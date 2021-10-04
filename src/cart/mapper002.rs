use super::Mirroring;

/// 002, UxROM
///
/// 0x8000-0xbfff: 16 KB switchable PRG banks,
/// 0xC000-0xffff: 16 KB PRG bank (fixed to the last bank),
/// 8 KB CHR
pub struct Mapper002 {
    prg_bank0: usize,
    prg_bank1: usize,

    mirroring: Mirroring,
}

impl Mapper002 {
    pub fn new(mirroring: Mirroring, prg_banks: usize) -> Self {
        assert!(
            prg_banks <= 256,
            "invalid banks for Mapper002, prg: {}",
            prg_banks
        );

        Self {
            prg_bank0: 0,
            prg_bank1: prg_banks - 1,

            mirroring,
        }
    }
}

impl super::Mapper for Mapper002 {
    fn read_prg(&self, prg: &[u8], addr: u16) -> u8 {
        match addr {
            0x8000..=0xbfff => prg[addr as usize - 0x8000 + self.prg_bank0 * 0x4000],
            0xc000..=0xffff => prg[addr as usize - 0xc000 + self.prg_bank1 * 0x4000],
            _ => unreachable!(),
        }
    }

    fn write_prg(&mut self, _prg: &mut [u8], addr: u16, data: u8) {
        match addr {
            0x8000..=0xffff => self.prg_bank0 = data as usize,
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
