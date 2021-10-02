use super::Mirroring;
use bit_field::BitField;

#[derive(Debug, PartialEq)]
enum PrgMode {
    SwapLow,
    SwapHigh,
}

#[derive(Debug, PartialEq)]
enum ChrMode {
    Low2KB,
    High2KB,
}

pub struct Mapper004 {
    prg_banks: [usize; 4],
    chr_banks: [usize; 8],
    prg_max: usize,

    bank_reg: u8,
    bank_regs: [u8; 8],
    prg_mode: PrgMode,
    chr_mode: ChrMode,

    irq_latch: u8,
    irq_counter: u8,
    irq_on: bool,
    irq_level: bool,

    mirroring: Mirroring,
}

impl Mapper004 {
    pub fn new(mirroring: Mirroring, prg_banks: usize) -> Self {
        assert!(prg_banks >= 1);
        let prg_max = prg_banks * 2;
        Self {
            prg_banks: [0, 1, prg_max - 2, prg_max - 1],
            chr_banks: [0; 8],
            prg_max,

            bank_reg: 0,
            bank_regs: [0; 8],
            prg_mode: PrgMode::SwapLow,
            chr_mode: ChrMode::Low2KB,

            irq_latch: 0,
            irq_counter: 0,
            irq_on: false,
            irq_level: false,

            mirroring,
        }
    }

    fn bank_select(&mut self, addr: u16, data: u8) {
        if addr % 2 == 0 {
            self.bank_reg = data.get_bits(0..3);
            self.prg_mode = data.get_bit(6).into();
            self.chr_mode = data.get_bit(7).into();
        } else {
            self.bank_regs[self.bank_reg as usize] = data;
        }

        self.update_banks();
    }

    fn update_banks(&mut self) {
        match self.prg_mode {
            PrgMode::SwapLow => {
                self.prg_banks[0] = (self.bank_regs[6] as usize & 0x3f) % self.prg_max;
                self.prg_banks[1] = (self.bank_regs[7] as usize & 0x3f) % self.prg_max;
                self.prg_banks[2] = self.prg_max - 2;
                self.prg_banks[3] = self.prg_max - 1;
            }
            PrgMode::SwapHigh => {
                self.prg_banks[0] = self.prg_max - 2;
                self.prg_banks[1] = (self.bank_regs[7] as usize & 0x3f) % self.prg_max;
                self.prg_banks[2] = (self.bank_regs[6] as usize & 0x3f) % self.prg_max;
                self.prg_banks[3] = self.prg_max - 1;
            }
        }
        match self.chr_mode {
            ChrMode::Low2KB => {
                self.chr_banks[0] = self.bank_regs[0] as usize & 0xfe;
                self.chr_banks[1] = self.chr_banks[0] + 1;
                self.chr_banks[2] = self.bank_regs[1] as usize & 0xfe;
                self.chr_banks[3] = self.chr_banks[2] + 1;
                self.chr_banks[4] = self.bank_regs[2] as usize;
                self.chr_banks[5] = self.bank_regs[3] as usize;
                self.chr_banks[6] = self.bank_regs[4] as usize;
                self.chr_banks[7] = self.bank_regs[5] as usize;
            }
            ChrMode::High2KB => {
                self.chr_banks[0] = self.bank_regs[2] as usize;
                self.chr_banks[1] = self.bank_regs[3] as usize;
                self.chr_banks[2] = self.bank_regs[4] as usize;
                self.chr_banks[3] = self.bank_regs[5] as usize;
                self.chr_banks[4] = self.bank_regs[0] as usize & 0xfe;
                self.chr_banks[5] = self.chr_banks[4] + 1;
                self.chr_banks[6] = self.bank_regs[1] as usize & 0xfe;
                self.chr_banks[7] = self.chr_banks[6] + 1;
            }
        }
    }
}

impl super::Mapper for Mapper004 {
    fn read_rpg(&self, rpg: &[u8], addr: u16) -> u8 {
        // 4 * 8KB prg banks
        let index = (addr >> 13) as usize & 0b11;
        let offset = addr as usize & 0x1fff;
        rpg[offset + self.prg_banks[index] * 0x2000]
    }

    fn write_rpg(&mut self, _rpg: &mut [u8], addr: u16, data: u8) {
        match addr {
            0x8000..=0x9fff => self.bank_select(addr, data),
            0xa000..=0xbfff => {
                if addr % 2 == 0 {
                    self.mirroring = if !data.get_bit(0) {
                        Mirroring::Vertical
                    } else {
                        Mirroring::Horizontal
                    };
                }
            }
            0xc000..=0xdfff => {
                if addr % 2 == 0 {
                    self.irq_latch = data;
                } else {
                    self.irq_counter = 0;
                }
            }
            0xe000..=0xffff => {
                self.irq_on = addr % 2 != 0;
                if !self.irq_on {
                    self.irq_level = false;
                }
            }
            _ => unreachable!(),
        }
    }

    fn read_chr(&self, chr: &[u8], addr: u16) -> u8 {
        // 8 * 1KB chr banks
        let index = (addr >> 10) as usize & 0b111;
        let offset = addr as usize & 0x3ff;
        chr[offset + self.chr_banks[index] * 0x400]
    }

    fn write_chr(&mut self, chr: &mut [u8], addr: u16, data: u8) {
        chr[addr as usize] = data;
    }

    fn update_scanline(&mut self) {
        if self.irq_counter == 0 {
            self.irq_counter = self.irq_latch;
        } else {
            self.irq_counter -= 1;

            if self.irq_on && self.irq_counter == 0 {
                self.irq_level = true;
            }
        }
    }

    fn poll_irq(&mut self) -> bool {
        std::mem::replace(&mut self.irq_level, false)
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
}

impl From<bool> for PrgMode {
    fn from(b: bool) -> Self {
        if !b {
            PrgMode::SwapLow
        } else {
            PrgMode::SwapHigh
        }
    }
}

impl From<bool> for ChrMode {
    fn from(b: bool) -> Self {
        if !b {
            ChrMode::Low2KB
        } else {
            ChrMode::High2KB
        }
    }
}
