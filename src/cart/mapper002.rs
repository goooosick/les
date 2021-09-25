use super::Mirroring;

/// 002, UxROM
///
/// 0x8000-0xbfff: 16 KB switchable PRG banks,
/// 0xC000-0xffff: 16 KB PRG bank (fixed to the last bank),
/// 8 KB CHR
pub struct Mapper002 {
    rpg_bank0: usize,
    rpg_bank1: usize,

    mirroring: Mirroring,
}

impl Mapper002 {
    pub fn new(mirroring: Mirroring, rpg_banks: usize) -> Self {
        assert!(
            rpg_banks <= 256,
            "invalid banks for Mapper002, rpg: {}",
            rpg_banks
        );

        Self {
            rpg_bank0: 0,
            rpg_bank1: rpg_banks - 1,

            mirroring,
        }
    }
}

impl super::Mapper for Mapper002 {
    fn read_rpg(&self, rpg: &[u8], addr: u16) -> u8 {
        match addr {
            0x8000..=0xbfff => rpg[addr as usize - 0x8000 + self.rpg_bank0 * 0x4000],
            0xc000..=0xffff => rpg[addr as usize - 0xc000 + self.rpg_bank1 * 0x4000],
            _ => unreachable!(),
        }
    }

    fn write_rpg(&mut self, _rpg: &mut [u8], addr: u16, data: u8) {
        match addr {
            0x8000..=0xffff => self.rpg_bank0 = data as usize,
            _ => unreachable!(),
        }
    }

    fn read_chr(&self, chr: &[u8], addr: u16) -> u8 {
        chr[addr as usize]
    }

    fn write_chr(&mut self, chr: &mut [u8], addr: u16, data: u8) {
        chr[addr as usize] = data;
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
}
