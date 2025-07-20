pub mod lidar;
pub mod wifi;

// Split resources between each of the tasks
use assign_resources::assign_resources;
use embassy_rp::{bind_interrupts, peripherals};

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
    }
}

// also bind interrupts
use embassy_rp::peripherals::{PIO0, UART1};
use embassy_rp::pio::InterruptHandler as PIOInterruptHandler;
use embassy_rp::uart::InterruptHandler as UARTInterruptHandler;

bind_interrupts!(pub struct Irqs {
    UART1_IRQ  => UARTInterruptHandler<UART1>;
    PIO0_IRQ_0 => PIOInterruptHandler<PIO0>;
});

// and create communication channels
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

static MOTE_TO_HOST: Channel<CriticalSectionRawMutex, mote_messages::MoteToHostMessage, 32> = Channel::new();
static HOST_TO_MOTE: Channel<CriticalSectionRawMutex, mote_messages::MoteToHostMessage, 32> = Channel::new();
