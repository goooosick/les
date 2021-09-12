/// 000, NROM
///
/// 16 KB or 32KB RPG,
/// 8 KB CHR
pub struct Mapper000 {
    rpg_bank1: usize,
}

impl Mapper000 {
    pub fn new(rpg_banks: usize) -> Self {
        assert!(
            rpg_banks == 1 || rpg_banks == 2,
            "invalid banks for Mapper000, rpg: {}",
            rpg_banks
        );

        Self {
            rpg_bank1: rpg_banks - 1,
        }
    }
}

impl super::Mapper for Mapper000 {
    fn read_rpg(&self, rpg: &[u8], addr: u16) -> u8 {
        match addr {
            0x8000..=0xbfff => rpg[addr as usize - 0x8000],
            0xc000..=0xffff => rpg[addr as usize - 0xc000 + self.rpg_bank1 * 0x4000],
            _ => unreachable!(),
        }
    }

    fn read_chr(&self, chr: &[u8], addr: u16) -> u8 {
        chr[addr as usize]
    }
}
