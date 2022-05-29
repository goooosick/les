use super::{Divider, LengthCounter};
use bit_field::BitField;

const SEQ: [u8; 32] = [
    0x0f, 0x0e, 0x0d, 0x0c, 0x0b, 0x0a, 0x09, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0x00,
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
];

#[derive(Debug)]
pub struct Triangle {
    step: usize,
    timer: Divider,
    len_counter: LengthCounter,

    linear_len: usize,
    linear_counter: usize,
    linear_reload: bool,
    linear_control: bool,
}

impl Triangle {
    pub fn new() -> Self {
        Self {
            step: 0,
            timer: Divider::new(),
            len_counter: LengthCounter::new(),

            linear_len: 0,
            linear_counter: 0,
            linear_reload: false,
            linear_control: false,
        }
    }
}

impl super::Channel for Triangle {
    fn sample(&mut self) -> u8 {
        SEQ[self.step]
    }

    fn tick(&mut self) {
        if self.timer.tick() && self.linear_counter != 0 && self.len_counter.count() != 0 {
            // silence when frequency is too high
            if self.timer.period() >= 2 {
                self.step = (self.step + 1) % 32;
            }
        }
    }

    fn tick_len(&mut self) {
        self.len_counter.tick();
    }

    fn tick_eve(&mut self) {
        if self.linear_reload {
            self.linear_counter = self.linear_len;
        } else if self.linear_counter > 0 {
            self.linear_counter -= 1;
        }

        if !self.linear_control {
            self.linear_reload = false;
        }
    }

    fn write_reg0(&mut self, data: u8) {
        self.len_counter.set_halt(data.get_bit(7));

        self.linear_control = data.get_bit(7);
        self.linear_len = data.get_bits(0..7) as usize;
    }

    fn write_reg1(&mut self, _: u8) {}

    fn write_reg2(&mut self, data: u8) {
        self.timer.set_period_low(data);
    }

    fn write_reg3(&mut self, data: u8) {
        self.timer.set_period_high(data);

        self.len_counter.load(data & 0xf8);
        self.linear_reload = true;
    }

    fn set_enable(&mut self, enable: bool) {
        self.len_counter.set_enable(enable);
    }

    fn enabled(&self) -> bool {
        self.len_counter.count() > 0
    }
}
