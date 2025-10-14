use defmt::{info, unwrap};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver as UsbDriver, Instance as UsbInstance};
use embassy_time::{Duration, Ticker, with_timeout};
use embassy_usb::UsbDevice;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;
use mote_messages::configuration::{host_to_mote, mote_to_host};
use mote_sansio_driver::{MoteConfigurationLink, SerialEndpoint};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use super::{Irqs, UsbSerialResources};
use crate::tasks::CONFIGURATION_STATE;
use crate::tasks::wifi::{WIFI_REQUEST_CONNECT, WIFI_REQUEST_RESCAN};

#[embassy_executor::task]
async fn usb_task(mut usb: UsbDevice<'static, UsbDriver<'static, USB>>) -> ! {
    usb.run().await
}

struct Disconnected {}

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => panic!("Buffer overflow"),
            EndpointError::Disabled => Disconnected {},
        }
    }
}

async fn handle_host_message(msg: host_to_mote::Message) {
    match msg {
        host_to_mote::Message::SetNetworkConnectionConfig(set_network_connection_config) => {
            WIFI_REQUEST_CONNECT.send(set_network_connection_config).await;
        }
        host_to_mote::Message::SetUID(set_uid) => {
            let mut configuration_state = CONFIGURATION_STATE.lock().await;
            configuration_state.uid = set_uid.uid.clone();
            info!("Setting UID: {}", configuration_state.uid);
        }
        host_to_mote::Message::RequestNetworkScan => {
            WIFI_REQUEST_RESCAN.signal(());
        }
    }
}

async fn handle_serial<'d, T: UsbInstance + 'd>(
    class: &mut CdcAcmClass<'d, UsbDriver<'d, T>>,
) -> Result<(), Disconnected> {
    // 64 bytes is the serial MTU
    let mut serial_buffer = [0; 64];

    // Periodic timer for telemetering state
    let mut ticker = Ticker::every(Duration::from_millis(500));

    // Link to the host
    let mut link = MoteConfigurationLink::new();

    loop {
        match select(class.read_packet(&mut serial_buffer), ticker.next()).await {
            Either::First(Ok(bytes_read)) => {
                link.handle_receive(&mut serial_buffer[..bytes_read]);
                while let Ok(Some(message)) = link.poll_receive() {
                    handle_host_message(message).await;
                }
                Ok(())
            }
            Either::First(Err(error)) => Err(error),
            Either::Second(_) => {
                if let Ok(configuration_state) =
                    with_timeout(Duration::from_millis(500), CONFIGURATION_STATE.lock()).await
                {
                    let message = mote_to_host::Message::State(configuration_state.clone());
                    link.send(SerialEndpoint, message).unwrap();
                }
                Ok(())
            }
        }?;

        while let Some(transmit) = link.poll_transmit() {
            if let Ok(res) = with_timeout(Duration::from_millis(500), class.write_packet(&transmit.payload)).await {
                res?;
            }
        }
    }
}

#[embassy_executor::task]
async fn usb_serial_task(spawner: Spawner, r: UsbSerialResources) {
    let driver = UsbDriver::new(r.usb, Irqs);

    // See https://github.com/raspberrypi/usb-pid for vid / pid
    let config = {
        let mut config = embassy_usb::Config::new(0x2E8A, 0x0009); // rpi CDC UART
        config.manufacturer = Some("Mote");
        config.product = Some("Mote Serial");
        config.serial_number = Some("12345678"); // TODO: fill this in
        config.max_power = 100;
        config.max_packet_size_0 = 64;
        config
    };

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut builder = {
        static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

        let builder = embassy_usb::Builder::new(
            driver,
            config,
            CONFIG_DESCRIPTOR.init([0; 256]),
            BOS_DESCRIPTOR.init([0; 256]),
            &mut [], // no msos descriptors
            CONTROL_BUF.init([0; 64]),
        );
        builder
    };

    // Create classes on the builder.
    let mut class = {
        static STATE: StaticCell<State> = StaticCell::new();
        let state = STATE.init(State::new());
        CdcAcmClass::new(&mut builder, state, 64)
    };

    // Build the builder.
    let usb = builder.build();

    unwrap!(spawner.spawn(usb_task(usb)));

    loop {
        class.wait_connection().await;
        info!("USB serial connected");
        let _ = handle_serial(&mut class).await;
        info!("USB serial disconnected");
    }
}

pub async fn init(spawner: Spawner, r: UsbSerialResources) {
    unwrap!(spawner.spawn(usb_serial_task(spawner, r)));
}
