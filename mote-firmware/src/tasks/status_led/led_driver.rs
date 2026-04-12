#![allow(dead_code)]
use embassy_rp::Peri;
use embassy_rp::dma::ChannelInstance;
use embassy_rp::interrupt::typelevel::Binding;
use embassy_rp::pio_programs::ws2812::{Grb, PioWs2812, PioWs2812Program};
use smart_leds::{RGB8, brightness, gamma};

const BRIGHTNESS: u8 = 8;

pub mod colors {
    use smart_leds::RGB8;
    pub const OFF: RGB8 = RGB8 { r: 0, g: 0, b: 0 };
    pub const RED: RGB8 = RGB8 { r: 255, g: 0, b: 0 };
    pub const GREEN: RGB8 = RGB8 { r: 0, g: 255, b: 0 };
    pub const BLUE: RGB8 = RGB8 { r: 0, g: 0, b: 255 };
    pub const YELLOW: RGB8 = RGB8 { r: 126, g: 129, b: 0 };
    pub const WHITE: RGB8 = RGB8 { r: 255, g: 255, b: 255 };
    pub const MAGENTA: RGB8 = RGB8 { r: 255, g: 0, b: 255 };
    pub const VIOLET: RGB8 = RGB8 { r: 125, g: 0, b: 255 };
    pub const ORANGE: RGB8 = RGB8 { r: 255, g: 125, b: 0 };
    pub const OCEAN: RGB8 = RGB8 { r: 0, g: 125, b: 255 };
    pub const CYAN: RGB8 = RGB8 { r: 0, g: 126, b: 129 };
    pub const MAROON: RGB8 = RGB8 { r: 128, g: 0, b: 0 };
    pub const PURPLE: RGB8 = RGB8 { r: 128, g: 0, b: 128 };
}

pub struct LedDriver<'d, P, const S: usize, const N: usize>
where
    P: embassy_rp::pio::Instance,
{
    ws2812: PioWs2812<'d, P, S, N, Grb>,
    frame_buf: [RGB8; N],
}

impl<'d, P, const S: usize, const N: usize> LedDriver<'d, P, S, N>
where
    P: embassy_rp::pio::Instance,
{
    pub fn new<D: ChannelInstance>(
        common: &mut embassy_rp::pio::Common<'d, P>,
        sm: embassy_rp::pio::StateMachine<'d, P, S>,
        dma: Peri<'d, D>,
        irq: impl Binding<D::Interrupt, embassy_rp::dma::InterruptHandler<D>> + 'd,
        pin: Peri<'d, impl embassy_rp::pio::PioPin>,
        program: &'d PioWs2812Program<'d, P>,
    ) -> Self {
        Self {
            ws2812: PioWs2812::new(common, sm, dma, irq, pin, program),
            frame_buf: [RGB8::default(); N],
        }
    }

    pub async fn flush(&mut self) {
        let corrected: [RGB8; N] = core::array::from_fn(|i| {
            let after_gamma: RGB8 = gamma(core::iter::once(self.frame_buf[i])).next().unwrap();
            brightness(core::iter::once(after_gamma), BRIGHTNESS).next().unwrap()
        });
        self.ws2812.write(&corrected).await;
    }

    pub async fn set_color(&mut self, color: RGB8) {
        self.frame_buf.fill(color);
        self.flush().await;
    }

    pub async fn set_colors(&mut self, colors: &[RGB8]) {
        let count = colors.len().min(N);
        self.frame_buf[..count].copy_from_slice(&colors[..count]);
        self.flush().await;
    }

    pub async fn set_one(&mut self, index: usize, color: RGB8) {
        if index < N {
            self.frame_buf[index] = color;
            self.flush().await;
        }
    }

    pub async fn off(&mut self) {
        self.set_color(colors::OFF).await;
    }

    pub fn num_leds(&self) -> usize {
        N
    }
}
