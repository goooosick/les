use super::Divider;
use bit_field::BitField;

const RATE: [usize; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

#[derive(Debug)]
pub struct Dmc {
    irq_on: bool,
    irq_level: Option<()>,
    looping: bool,

    sample_start_address: u16,
    sample_address: u16,
    sample_len: u16,
    sample_remain: u16,
    sample_request: Option<u16>,

    timer: Divider,
    bits_shifter: u8,
    bits_remain: u8,

    output: u8,
}

impl Dmc {
    pub fn new() -> Self {
        Self {
            irq_on: false,
            irq_level: None,
            looping: false,

            sample_start_address: 0,
            sample_address: 0,
            sample_len: 0,
            sample_remain: 0,
            sample_request: None,

            timer: Divider::new(),
            bits_shifter: 0,
            bits_remain: 0,

            output: 0,
        }
    }

    pub fn read_sample(&mut self) -> Option<u16> {
        self.sample_request.take()
    }

    pub fn write_sample(&mut self, data: u8) {
        self.bits_shifter = data;
        self.bits_remain = 8;
    }

    pub fn irq(&self) -> bool {
        self.irq_level.is_some()
    }

    fn restart(&mut self) {
        self.sample_remain = self.sample_len;
        self.sample_address = self.sample_start_address;
    }
}

impl super::Channel for Dmc {
    fn sample(&mut self) -> u8 {
        self.output
    }

    fn tick(&mut self) {
        if self.sample_remain > 0 && self.bits_remain == 0 {
            self.sample_request = Some(self.sample_address);

            if self.sample_address == 0xffff {
                self.sample_address = 0x8000;
            } else {
                self.sample_address += 1;
            }
            self.sample_remain -= 1;

            if self.sample_remain == 0 {
                if self.looping {
                    self.restart();
                } else if self.irq_on {
                    self.irq_level = Some(());
                }
            }
        }

        if self.timer.tick() {
            if self.bits_remain > 0 {
                if self.bits_shifter.get_bit(0) {
                    if self.output <= 125 {
                        self.output += 2;
                    }
                } else {
                    if self.output >= 2 {
                        self.output -= 2;
                    }
                }

                self.bits_shifter >>= 1;
                self.bits_remain -= 1;
            }
        }
    }

    fn tick_len(&mut self) {}

    fn tick_eve(&mut self) {}

    fn write_reg0(&mut self, data: u8) {
        self.irq_on = data.get_bit(7);
        self.looping = data.get_bit(6);
        self.timer
            .set_period(RATE[data.get_bits(0..4) as usize] / 2);

        if !self.irq_on {
            self.irq_level.take();
        }
    }

    fn write_reg1(&mut self, data: u8) {
        self.output = data & 0x7f;
    }

    fn write_reg2(&mut self, data: u8) {
        self.sample_start_address = 0xc000 + data as u16 * 64;
    }

    fn write_reg3(&mut self, data: u8) {
        self.sample_len = data as u16 * 16 + 1;
    }

    fn set_enable(&mut self, enable: bool) {
        if !enable {
            self.sample_remain = 0;
        } else {
            if self.sample_remain == 0 {
                self.restart();
            }
        }

        self.irq_level.take();
    }

    fn enabled(&self) -> bool {
        self.sample_remain > 0
    }
}
