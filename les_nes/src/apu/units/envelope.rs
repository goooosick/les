use super::Divider;
use bit_field::BitField;

#[derive(Debug)]
pub struct Envelope {
    divider: Divider,
    volume: u8,
    counter: u8,
    looping: bool,
    constant: bool,
    restart: bool,
}

impl Envelope {
    pub fn new() -> Self {
        Self {
            divider: Divider::new(),
            volume: 0,
            counter: 0,
            looping: false,
            constant: true,
            restart: false,
        }
    }

    pub fn tick(&mut self) {
        if self.restart {
            self.restart = false;
            self.counter = 15;
            self.divider.reset();
        } else {
            if self.divider.tick() {
                if self.counter > 0 {
                    self.counter -= 1;
                } else if self.looping {
                    self.counter = 15;
                }
            }
        }
    }

    pub fn volume(&self) -> u8 {
        if self.constant {
            self.volume
        } else {
            self.counter
        }
    }

    pub fn load(&mut self, data: u8) {
        self.looping = data.get_bit(5);
        self.constant = data.get_bit(4);
        self.volume = data.get_bits(0..4);
        self.divider.set_period(self.volume as usize + 1);
    }

    pub fn restart(&mut self) {
        self.restart = true;
    }
}
