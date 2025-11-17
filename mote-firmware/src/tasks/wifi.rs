pub mod connection_manager;

mod mdns;
mod tcp_server;
mod udp_server;

use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use embassy_executor::Spawner;
use embassy_net::{Config, StackResources};
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::Pio;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use mote_messages::configuration::mote_to_host::{BIT, BITResult};
use mote_messages::runtime::{host_to_mote, mote_to_host};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use super::{Cyw43Resources, Irqs};
use crate::helpers::update_bit_result;
use crate::tasks::CONFIGURATION_STATE;

pub static MOTE_TO_HOST_DATA_OFFLOAD: Channel<CriticalSectionRawMutex, mote_to_host::data_offload::Message, 32> =
    Channel::new();
pub static HOST_TO_MOTE_COMMAND: Channel<CriticalSectionRawMutex, host_to_mote::Message, 32> = Channel::new();

#[embassy_executor::task]
async fn cyw43_task(runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

pub async fn init(spawner: Spawner, r: Cyw43Resources) {
    // Init BIT
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        let init = BIT {
            name: heapless::String::try_from("Init").expect("Failed to assign name to BIT"),
            result: BITResult::Waiting,
        };
        let connection = BIT {
            name: heapless::String::try_from("Connected to Network").expect("Failed to assign name to BIT"),
            result: BITResult::Waiting,
        };
        let ip_v4 = BIT {
            name: heapless::String::try_from("IPV4 UP").expect("Failed to assign name to BIT"),
            result: BITResult::Waiting,
        };
        let multicast = BIT {
            name: heapless::String::try_from("mDNS UP").expect("Failed to assign name to BIT"),
            result: BITResult::Waiting,
        };
        let client = BIT {
            name: heapless::String::try_from("Client Connected").expect("Failed to assign name to BIT"),
            result: BITResult::Waiting,
        };
        for test in [init, connection, ip_v4, multicast, client] {
            configuration_state
                .built_in_test
                .wifi
                .push(test)
                .expect("Failed to add test");
        }
    }

    // let fw = include_bytes!("../../cyw43-firmware/43439A0.bin");
    // let clm = include_bytes!("../../cyw43-firmware/43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares
    // independently at hardcoded addresses, instead of baking them into the
    // program with `include_bytes!`:     probe-rs download
    // ../../cyw43-firmware/43439A0.bin --binary-format bin --chip RP235x
    // --base-address 0x10100000     probe-rs download
    // ../../cyw43-firmware/43439A0_clm.bin --binary-format bin --chip RP235x
    // --base-address 0x10140000
    let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let pwr = Output::new(r.pwr, Level::Low);
    let cs = Output::new(r.cs, Level::High);
    let mut pio = Pio::new(r.pio, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        r.dio,
        r.clk,
        r.dma,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    spawner.spawn(cyw43_task(runner).unwrap());

    control.init(clm).await;
    control.set_power_management(cyw43::PowerManagementMode::None).await;

    // Update init state
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        update_bit_result(&mut configuration_state.built_in_test.wifi, "Init", BITResult::Pass);
    }

    let config = Config::dhcpv4(Default::default());

    // Generate random seed
    let seed = RoscRng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(net_device, config, RESOURCES.init(StackResources::new()), seed);

    // Start connection manager task
    spawner.spawn(connection_manager::connection_manager_task(control).unwrap());

    // Start network task
    spawner.spawn(net_task(runner).unwrap());

    // Start mdns responder
    spawner.spawn(mdns::mdns_task(stack).unwrap());

    // Start the tcp command server
    spawner.spawn(tcp_server::tcp_server_task(stack).unwrap());

    // Start the udp data offload server
    spawner.spawn(udp_server::udp_server_task(stack).unwrap());
}
