use super::{Divider, Envelope, LengthCounter, Sweep};
use bit_field::BitField;

const DUTY: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0],
    [0, 1, 1, 0, 0, 0, 0, 0],
    [0, 1, 1, 1, 1, 0, 0, 0],
    [1, 0, 0, 1, 1, 1, 1, 1],
];

#[derive(Debug)]
pub struct Pulse {
    len_counter: LengthCounter,
    envelope: Envelope,
    sweep: Sweep,

    timer: Divider,
    duty: usize,
    step: usize,
}

impl Pulse {
    pub fn new(channel2: bool) -> Self {
        Self {
            len_counter: LengthCounter::new(),
            envelope: Envelope::new(),
            sweep: Sweep::new(channel2 as usize),

            timer: Divider::new(),
            duty: 0,
            step: 0,
        }
    }
}

impl super::Channel for Pulse {
    fn sample(&mut self) -> u8 {
        let duty = DUTY[self.duty][self.step];
        self.envelope.volume() * duty * self.len_counter.count() * (!self.sweep.muting() as u8)
    }

    fn tick(&mut self) {
        if self.timer.tick() {
            self.step = (self.step + 1) % 8;
        }
    }

    fn tick_len(&mut self) {
        self.len_counter.tick();
        self.sweep.tick(&mut self.timer);
    }

    fn tick_eve(&mut self) {
        self.envelope.tick();
    }

    fn write_reg0(&mut self, data: u8) {
        self.duty = data.get_bits(6..8) as usize;

        self.len_counter.set_halt(data.get_bit(5));
        self.envelope.load(data.get_bits(0..6));
    }

    fn write_reg1(&mut self, data: u8) {
        self.sweep.load(data);
    }

    fn write_reg2(&mut self, data: u8) {
        self.timer.set_period_low(data);
    }

    fn write_reg3(&mut self, data: u8) {
        self.envelope.restart();
        self.len_counter.load(data & 0xf8);

        self.step = 0;
        self.timer.set_period_high(data);
    }

    fn set_enable(&mut self, enable: bool) {
        self.len_counter.set_enable(enable);
    }

    fn enabled(&self) -> bool {
        self.len_counter.count() > 0
    }
}
