#[derive(Debug)]
pub struct Divider {
    period: usize,
    counter: usize,
}

impl Divider {
    pub fn new() -> Self {
        Self {
            period: 0,
            counter: 0,
        }
    }

    pub fn with_period(period: usize) -> Self {
        Self { period, counter: 0 }
    }

    pub fn tick(&mut self) -> bool {
        if self.counter > 0 {
            self.counter -= 1;
            false
        } else {
            self.counter = self.period;
            true
        }
    }

    pub fn reset(&mut self) {
        self.counter = self.period;
    }

    pub fn set_period(&mut self, period: usize) {
        self.period = period;
    }

    pub fn period(&self) -> usize {
        self.period
    }

    pub fn count(&self) -> usize {
        self.counter
    }
}
