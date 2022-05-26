use super::Mirroring;

/// 000, NROM
///
/// 16 KB or 32KB PRG,
/// 8 KB CHR
pub struct Mapper000 {
    prg_bank1: usize,
    mirroring: Mirroring,
}

impl Mapper000 {
    pub fn new(mirroring: Mirroring, prg_banks: usize) -> Self {
        assert!(
            prg_banks == 1 || prg_banks == 2,
            "invalid banks for Mapper000, prg: {}",
            prg_banks
        );

        Self {
            prg_bank1: prg_banks - 1,
            mirroring,
        }
    }
}

impl super::Mapper for Mapper000 {
    fn read_prg(&self, prg: &[u8], addr: u16) -> u8 {
        match addr {
            0x8000..=0xbfff => prg[addr as usize - 0x8000],
            0xc000..=0xffff => prg[addr as usize - 0xc000 + self.prg_bank1 * 0x4000],
            _ => unreachable!(),
        }
    }

    fn read_chr(&self, chr: &[u8], addr: u16) -> u8 {
        chr[addr as usize]
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
}
