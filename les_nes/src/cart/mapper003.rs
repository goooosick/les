use super::Mirroring;

/// 003, CNROM
///
/// 16 KB or 32KB PRG,
/// 8 KB switchable CHR banks (up to 256)
pub struct Mapper003 {
    prg_bank1: usize,
    chr_bank: usize,
    chr_banks: usize,

    mirroring: Mirroring,
}

impl Mapper003 {
    pub fn new(mirroring: Mirroring, prg_banks: usize, chr_banks: usize) -> Self {
        assert!(
            (prg_banks == 1 || prg_banks == 2) && chr_banks <= 256,
            "invalid banks for Mapper003, prg: {}, chr: {}",
            prg_banks,
            chr_banks
        );

        Self {
            prg_bank1: prg_banks - 1,
            chr_bank: 0,
            chr_banks,

            mirroring,
        }
    }
}

impl super::Mapper for Mapper003 {
    fn read_prg(&self, prg: &[u8], addr: u16) -> u8 {
        match addr {
            0x8000..=0xbfff => prg[addr as usize - 0x8000],
            0xc000..=0xffff => prg[addr as usize - 0xc000 + self.prg_bank1 * 0x4000],
            _ => unreachable!(),
        }
    }

    fn write_prg(&mut self, _prg: &mut [u8], addr: u16, data: u8) {
        match addr {
            0x8000..=0xffff => self.chr_bank = data as usize % self.chr_banks,
            _ => unreachable!(),
        }
    }

    fn read_chr(&self, chr: &[u8], addr: u16) -> u8 {
        chr[self.chr_bank * 0x2000 + addr as usize]
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
}
