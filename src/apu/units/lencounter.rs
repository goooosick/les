const LEN_TABLE: [u8; 32] = [
    0x0a, 0xfe, 0x14, 0x02, 0x28, 0x04, 0x50, 0x06, 0xa0, 0x08, 0x3c, 0x0a, 0x0e, 0x0c, 0x1a, 0x0e,
    0x0c, 0x10, 0x18, 0x12, 0x30, 0x14, 0x60, 0x16, 0xc0, 0x18, 0x48, 0x1a, 0x10, 0x1c, 0x20, 0x1e,
];

#[derive(Debug)]
pub struct LengthCounter {
    counter: u8,
    enabled: bool,
    halt: bool,
}

impl LengthCounter {
    pub fn new() -> Self {
        Self {
            counter: 0,
            enabled: false,
            halt: false,
        }
    }

    pub fn count(&self) -> u8 {
        (self.counter != 0) as u8
    }

    pub fn tick(&mut self) {
        if !self.halt && self.counter > 0 {
            self.counter -= 1;
        }
    }

    pub fn set_enable(&mut self, enable: bool) {
        self.enabled = enable;
        if !self.enabled {
            self.counter = 0;
        }
    }

    pub fn set_halt(&mut self, halt: bool) {
        self.halt = halt;
    }

    pub fn load(&mut self, data: u8) {
        if self.enabled {
            self.counter = LEN_TABLE[(data >> 3) as usize];
        }
    }
}
