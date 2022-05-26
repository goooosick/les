use super::Mirroring;
use bit_field::BitField;

#[derive(Debug, PartialEq, Eq)]
enum PRGMode {
    Full,
    FixedFirst,
    FixedLast,
}

#[derive(Debug, PartialEq, Eq)]
enum CHRMode {
    Full,
    Half,
}

pub struct Mapper001 {
    prg_bank0: usize,
    prg_bank1: usize,
    prg_banks: usize,
    chr_bank0: usize,
    chr_bank1: usize,

    step: u8,
    shifter: u8,
    prg_mode: PRGMode,
    chr_mode: CHRMode,
    enable_ram: bool,

    mirroring: Mirroring,
}

impl Mapper001 {
    pub fn new(mirroring: Mirroring, prg_banks: usize) -> Self {
        Self {
            prg_bank0: 0,
            prg_bank1: prg_banks - 1,
            prg_banks,
            chr_bank0: 0,
            chr_bank1: 1,

            step: 0,
            shifter: 0,
            prg_mode: PRGMode::FixedLast,
            chr_mode: CHRMode::Full,
            enable_ram: false,

            mirroring,
        }
    }

    pub fn handle_write(&mut self, addr: u16) {
        let bank_bits = self.shifter as usize;

        match addr {
            0x8000..=0x9fff => {
                self.mirroring = match self.shifter.get_bits(0..=1) {
                    0b00 => Mirroring::SingleScreen0,
                    0b01 => Mirroring::SingleScreen1,
                    0b10 => Mirroring::Vertical,
                    0b11 => Mirroring::Horizontal,
                    _ => unreachable!(),
                };
                self.prg_mode = PRGMode::from_bits(self.shifter.get_bits(2..=3));
                self.chr_mode = CHRMode::from_bits(self.shifter.get_bits(4..=4));
            }
            0xa000..=0xbfff => {
                if self.chr_mode == CHRMode::Full {
                    self.chr_bank0 = bank_bits & 0b11110;
                    self.chr_bank1 = self.chr_bank0 + 1;
                } else {
                    self.chr_bank0 = bank_bits;
                }
            }
            0xc000..=0xdfff => {
                if self.chr_mode != CHRMode::Full {
                    self.chr_bank1 = bank_bits;
                }
            }
            0xe000..=0xffff => {
                match self.prg_mode {
                    PRGMode::Full => {
                        self.prg_bank0 = bank_bits & 0b01110;
                        self.prg_bank1 = self.prg_bank0 + 1;
                    }
                    PRGMode::FixedFirst => {
                        self.prg_bank0 = 0;
                        self.prg_bank1 = bank_bits & 0b01111;
                    }
                    PRGMode::FixedLast => {
                        self.prg_bank0 = bank_bits & 0b01111;
                        self.prg_bank1 = self.prg_banks - 1;
                    }
                }

                self.enable_ram = !bank_bits.get_bit(5);
            }
            _ => unreachable!(),
        }
    }
}

impl super::Mapper for Mapper001 {
    fn read_prg(&self, prg: &[u8], addr: u16) -> u8 {
        match addr {
            0x8000..=0xbfff => prg[addr as usize - 0x8000 + self.prg_bank0 * 0x4000],
            0xc000..=0xffff => prg[addr as usize - 0xc000 + self.prg_bank1 * 0x4000],
            _ => unreachable!(),
        }
    }

    fn write_prg(&mut self, _prg: &mut [u8], addr: u16, data: u8) {
        if data.get_bit(7) {
            self.step = 0;
            self.shifter = 0;

            self.prg_mode = PRGMode::FixedLast;
            self.prg_bank1 = self.prg_banks - 1;
        } else {
            self.shifter >>= 1;
            self.shifter.set_bit(4, data.get_bit(0));

            self.step += 1;
            if self.step == 5 {
                self.handle_write(addr);

                self.step = 0;
                self.shifter = 0;
            }
        }
    }

    fn read_chr(&self, chr: &[u8], addr: u16) -> u8 {
        match addr {
            0x0000..=0x0fff => chr[(addr & 0x0fff) as usize + self.chr_bank0 * 0x1000],
            0x1000..=0x1fff => chr[(addr & 0x0fff) as usize + self.chr_bank1 * 0x1000],
            _ => unreachable!(),
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
}

impl PRGMode {
    fn from_bits(bits: u8) -> Self {
        match bits {
            0b00 | 0b01 => PRGMode::Full,
            0b10 => PRGMode::FixedFirst,
            0b11 => PRGMode::FixedLast,
            _ => unreachable!(),
        }
    }
}

impl CHRMode {
    fn from_bits(bits: u8) -> Self {
        match bits {
            0b00 => CHRMode::Full,
            0b01 => CHRMode::Half,
            _ => unreachable!(),
        }
    }
}
