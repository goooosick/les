use super::{Divider, Envelope, LengthCounter};
use bit_field::BitField;

const PERIOD: [usize; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

#[derive(Debug)]
pub struct Noise {
    envelope: Envelope,
    len_counter: LengthCounter,

    timer: Divider,
    lfsr: u16,
    bit_index: usize,
}

impl Noise {
    pub fn new() -> Self {
        Self {
            envelope: Envelope::new(),
            len_counter: LengthCounter::new(),

            timer: Divider::new(),
            lfsr: 1,
            bit_index: 1,
        }
    }
}

impl super::Channel for Noise {
    fn sample(&mut self) -> u8 {
        self.envelope.volume() * self.len_counter.count() * (!self.lfsr.get_bit(0) as u8)
    }

    fn tick(&mut self) {
        if self.timer.tick() {
            let feed = self.lfsr.get_bit(0) ^ self.lfsr.get_bit(self.bit_index);
            self.lfsr >>= 1;
            self.lfsr.set_bit(14, feed);
        }
    }

    fn tick_len(&mut self) {
        self.len_counter.tick();
    }

    fn tick_eve(&mut self) {
        self.envelope.tick();
    }

    fn write_reg0(&mut self, data: u8) {
        self.len_counter.set_halt(data.get_bit(5));
        self.envelope.load(data.get_bits(0..6));
    }

    fn write_reg1(&mut self, _: u8) {}

    fn write_reg2(&mut self, data: u8) {
        self.bit_index = data.get_bit(7) as usize * 5 + 1;
        self.timer.set_period(PERIOD[data.get_bits(0..4) as usize]);
    }

    fn write_reg3(&mut self, data: u8) {
        self.envelope.restart();
        self.len_counter.load(data & 0xf8);
    }

    fn set_enable(&mut self, enable: bool) {
        self.len_counter.set_enable(enable);
    }

    fn enabled(&self) -> bool {
        self.len_counter.count() > 0
    }
}
