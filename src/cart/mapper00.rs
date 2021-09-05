pub struct Mapper00 {
    offset: usize,
}

impl Mapper00 {
    pub fn new(rpg_banks: usize) -> Self {
        assert!(
            rpg_banks == 1 || rpg_banks == 2,
            "invalid rpg banks - {} for Mapper00",
            rpg_banks
        );

        Self {
            offset: (rpg_banks - 1) * 0x4000,
        }
    }
}

impl super::Mapper for Mapper00 {
    fn read_rpg(&self, rpg: &[u8], addr: u16) -> u8 {
        match addr {
            0x8000..=0xbfff => rpg[addr as usize - 0x8000],
            0xc000..=0xffff => rpg[addr as usize - 0xc000 + self.offset],
            _ => unreachable!(),
        }
    }

    fn read_chr(&self, chr: &[u8], addr: u16) -> u8 {
        chr[addr as usize]
    }
}
