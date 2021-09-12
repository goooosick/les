// 003, CNROM
///
/// 16 KB or 32KB RPG,
/// 8 KB switchable CHR banks (up to 256)
pub struct Mapper003 {
    rpg_bank1: usize,
    chr_bank: usize,
}

impl Mapper003 {
    pub fn new(rpg_banks: usize, chr_banks: usize) -> Self {
        assert!(
            (rpg_banks == 1 || rpg_banks == 2) && chr_banks <= 256,
            "invalid banks for Mapper003, rpg: {}, chr: {}",
            rpg_banks,
            chr_banks
        );

        Self {
            rpg_bank1: rpg_banks - 1,
            chr_bank: 0,
        }
    }
}

impl super::Mapper for Mapper003 {
    fn read_rpg(&self, rpg: &[u8], addr: u16) -> u8 {
        match addr {
            0x8000..=0xbfff => rpg[addr as usize - 0x8000],
            0xc000..=0xffff => rpg[addr as usize - 0xc000 + self.rpg_bank1 * 0x4000],
            _ => unreachable!(),
        }
    }

    fn write_rpg(&mut self, _rpg: &mut [u8], addr: u16, data: u8) {
        match addr {
            0x8000..=0xffff => self.chr_bank = data as usize,
            _ => unreachable!(),
        }
    }

    fn read_chr(&self, chr: &[u8], addr: u16) -> u8 {
        chr[self.chr_bank * 0x2000 + addr as usize]
    }
}
