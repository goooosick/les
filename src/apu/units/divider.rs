#[derive(Debug)]
pub struct Divider {
    period: usize,
    raw_period: usize,
    counter: usize,
}

impl Divider {
    pub fn new() -> Self {
        Self {
            period: 0,
            raw_period: 0,
            counter: 0,
        }
    }

    pub fn tick(&mut self) -> bool {
        self.counter = self.counter.saturating_sub(1);
        if self.counter == 0 {
            self.counter = self.period;
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.counter = self.period;
    }

    pub fn set_period(&mut self, period: usize) {
        self.period = period;
        self.raw_period = period;
    }

    pub fn set_raw_period(&mut self, period: usize) {
        self.raw_period = period;
        self.period = period + 1;
    }

    pub fn set_period_low(&mut self, data: u8) {
        let p = self.raw_period & 0xff00;
        self.raw_period = p | data as usize;
        self.period = self.raw_period + 1;
    }

    pub fn set_period_high(&mut self, data: u8) {
        let p = self.raw_period & 0x00ff;
        self.raw_period = p | ((data as usize & 0b111) << 8);
        self.period = self.raw_period + 1;
    }

    pub fn period(&self) -> usize {
        self.raw_period
    }
}
