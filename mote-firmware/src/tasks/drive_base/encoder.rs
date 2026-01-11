// From: https://github.com/embassy-rs/embassy/blob/main/examples/rp235x/src/bin/pio_rotary_encoder_rxf.rs

use embassy_rp::gpio::Pull;
use embassy_rp::pio::program::pio_asm;
use embassy_rp::pio::{Common, Config, FifoJoin, Instance, PioPin, ShiftDirection, StateMachine};
use embassy_rp::{Peri, pio};
use fixed::traits::ToFixed;

pub struct PioEncoder<'d, T: Instance, const SM: usize> {
    sm: StateMachine<'d, T, SM>,
}

impl<'d, T: Instance, const SM: usize> PioEncoder<'d, T, SM> {
    pub fn new(
        pio: &mut Common<'d, T>,
        mut sm: StateMachine<'d, T, SM>,
        pin_a: Peri<'d, impl PioPin>,
        pin_b: Peri<'d, impl PioPin>,
    ) -> Self {
        let mut pin_a = pio.make_pio_pin(pin_a);
        let mut pin_b = pio.make_pio_pin(pin_b);
        pin_a.set_pull(Pull::Up);
        pin_b.set_pull(Pull::Up);

        sm.set_pin_dirs(pio::Direction::In, &[&pin_a, &pin_b]);

        let prg = pio_asm!(
            "start:"
            // encoder count is stored in X
            "mov isr, x"
            // and then moved to the RX FIFO register
            "mov rxfifo[0], isr"

            // wait for encoder transition
            "wait 1 pin 1"
            "wait 0 pin 1"

            "set y, 0"
            "mov y, pins[1]"

            // update X depending on pin 1
            "jmp !y decr"

            // this is just a clever way of doing x++
            "mov x, ~x"
            "jmp x--, incr"
            "incr:"
            "mov x, ~x"
            "jmp start"

            // and this is x--
            "decr:"
            "jmp x--, start"
        );

        let mut cfg = Config::default();
        cfg.set_in_pins(&[&pin_a, &pin_b]);
        cfg.fifo_join = FifoJoin::RxAsStatus;
        cfg.shift_in.direction = ShiftDirection::Left;
        cfg.clock_divider = 0x0200.to_fixed();
        cfg.use_program(&pio.load_program(&prg.program), &[]);
        sm.set_config(&cfg);

        sm.set_enable(true);
        Self { sm }
    }

    pub async fn read(&mut self) -> i32 {
        self.sm.get_rxf_entry(0) as i32
    }
}
