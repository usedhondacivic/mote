pub mod lidar;
pub mod usb_serial;
pub mod wifi;

// Split resources between each of the tasks
use assign_resources::assign_resources;
use embassy_rp::{Peri, bind_interrupts, peripherals};

assign_resources! {
    wifi: Cyw43Resources{
        pwr: PIN_23,
        cs: PIN_25,
        pio: PIO0,
        dio: PIN_24,
        clk: PIN_29,
        dma: DMA_CH0
    },
    lidar_uart: RplidarC1Resources{
        uart: UART1,
        tx: PIN_4,
        rx: PIN_5,
        tx_dma: DMA_CH1,
        rx_dma: DMA_CH2
    },
    usb_serial: UsbSerialResources{
        usb: USB
    }
}

// also bind interrupts
use embassy_rp::peripherals::{PIO0, UART1, USB};
use embassy_rp::pio::InterruptHandler as PIOInterruptHandler;
use embassy_rp::uart::InterruptHandler as UARTInterruptHandler;
use embassy_rp::usb::InterruptHandler as USBInterruptHandler;

bind_interrupts!(pub struct Irqs {
    UART1_IRQ  => UARTInterruptHandler<UART1>;
    PIO0_IRQ_0 => PIOInterruptHandler<PIO0>;
    USBCTRL_IRQ => USBInterruptHandler<USB>;
});

// and create communication channels
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

static MOTE_TO_HOST: Channel<CriticalSectionRawMutex, mote_messages::runtime::mote_to_host::Message, 32> =
    Channel::new();
static HOST_TO_MOTE: Channel<CriticalSectionRawMutex, mote_messages::runtime::host_to_mote::Message, 32> =
    Channel::new();

// and init global configuration state
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use heapless::Vec;
use mote_messages::configuration::mote_to_host::{BITCollection, State};

pub static CONFIGURATION_STATE: Mutex<ThreadModeRawMutex, State> = Mutex::new(State {
    uid: heapless::String::<20>::new(),
    current_network_connection: None,
    available_network_connections: Vec::new(),
    built_in_test: BITCollection {
        lidar: Vec::new(),
        imu: Vec::new(),
        wifi: Vec::new(),
        encoders: Vec::new(),
    },
});
