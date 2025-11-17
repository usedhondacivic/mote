pub mod drive_base;
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
    },
    usb_serial: UsbSerialResources{
        usb: USB
    },
    left_encoder: LeftEncoderResources{
        pio: PIO1,
        phase_a: PIN_6,
        phase_b: PIN_7,
    },
    right_encoder: RightEncoderResources{
        pio: PIO2,
        phase_a: PIN_8,
        phase_b: PIN_9,
    },
    drive_base: DriveBaseResources{
        pwm: PWM_SLICE5,
        left_motor: PIN_10,
        right_motor: PIN_11,
    }
}

// also bind interrupts
use embassy_rp::peripherals::{PIO0, PIO1, PIO2, UART1, USB};
use embassy_rp::pio::InterruptHandler as PIOInterruptHandler;
use embassy_rp::uart::BufferedInterruptHandler as UARTInterruptHandler;
use embassy_rp::usb::InterruptHandler as USBInterruptHandler;

bind_interrupts!(pub struct Irqs {
    UART1_IRQ  => UARTInterruptHandler<UART1>;
    PIO0_IRQ_0 => PIOInterruptHandler<PIO0>;
    PIO1_IRQ_0 => PIOInterruptHandler<PIO1>;
    PIO2_IRQ_0 => PIOInterruptHandler<PIO2>;
    USBCTRL_IRQ => USBInterruptHandler<USB>;
});

// and init global configuration state
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use heapless::Vec;
use mote_messages::configuration::mote_to_host::{BITCollection, State, UID};

pub static CONFIGURATION_STATE: Mutex<CriticalSectionRawMutex, State> = Mutex::new(State {
    uid: UID::new(),
    current_network_connection: None,
    available_network_connections: Vec::new(),
    built_in_test: BITCollection {
        lidar: Vec::new(),
        imu: Vec::new(),
        wifi: Vec::new(),
        encoders: Vec::new(),
    },
    ip: None,
});
