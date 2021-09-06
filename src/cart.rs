use bit_field::BitField;
use std::path::Path;

mod mapper00;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    SingleScreen,
    FourScreen,
}

const EXPANSION_ROM_SIZE: usize = 0x1fe0;
const RPG_RAM_SIZE: usize = 0x2000;

pub struct Cartridge {
    expansion: Box<[u8; EXPANSION_ROM_SIZE]>,
    rpg_ram: Box<[u8; RPG_RAM_SIZE]>,
    rpg_rom: Vec<u8>,
    chr_rom: Vec<u8>,

    mirroring: Mirroring,
    mapper: Box<dyn Mapper>,
}

impl Cartridge {
    pub fn load(file: impl AsRef<Path>) -> Option<Self> {
        let data = std::fs::read(file).ok()?;
        if data[..4] != [b'N', b'E', b'S', 0x1a] {
            return None;
        }
        if data[7] & 0b1100 == 0b1100 {
            panic!("iNes 2.0");
        }

        let f6 = data[6];
        let _ram = f6.get_bit(1);
        let trainer = f6.get_bit(2);
        let mirroring = {
            match (f6 & 0b01) | ((f6 >> 2) & 0b10) {
                0b00 => Mirroring::Horizontal,
                0b01 => Mirroring::Vertical,
                0b10 => Mirroring::SingleScreen,
                0b11 => Mirroring::FourScreen,
                _ => unreachable!(),
            }
        };

        let mapper_type = (data[7] & 0xf0) | (f6 >> 4);

        let offset = 0x10 + (trainer as usize) * 0x200;
        let rpg_banks = data[4] as usize;
        let rpg_rom = data[offset..][..(rpg_banks * 0x4000)].to_vec();

        let offset = offset + rpg_rom.len();
        let chr_banks = data[5] as usize;
        let chr_rom = data[offset..][..(chr_banks * 0x2000)].to_vec();

        println!("MAPPER: {:02}", mapper_type);
        println!("RPG ROM: {} * 16KB = {}", rpg_banks, rpg_rom.len());
        println!("CHR ROM: {} * 8KB = {}", chr_banks, chr_rom.len());
        println!("MIRRORING: {:?}", mirroring);

        Some(Self {
            expansion: Box::new([0u8; EXPANSION_ROM_SIZE]),
            rpg_ram: Box::new([0u8; RPG_RAM_SIZE]),
            rpg_rom,
            chr_rom,

            mirroring,
            mapper: Box::new(match mapper_type {
                0x00 => mapper00::Mapper00::new(rpg_banks),
                _ => unimplemented!("unimplemented mapper type: {}", mapper_type),
            }),
        })
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x4020..=0x5fff => self.expansion[addr as usize - 0x4020],
            0x6000..=0x7fff => self.rpg_ram[addr as usize - 0x6000],
            0x8000..=0xffff => self.mapper.read_rpg(self.rpg_rom.as_ref(), addr),
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x4020..=0x5fff => self.expansion[addr as usize - 0x4020] = data,
            0x6000..=0x7fff => self.rpg_ram[addr as usize - 0x6000] = data,
            0x8000..=0xffff => self.mapper.write_rpg(self.rpg_rom.as_mut(), addr, data),
            _ => unreachable!(),
        }
    }

    pub fn read_chr(&self, addr: u16) -> u8 {
        self.mapper.read_chr(self.chr_rom.as_ref(), addr)
    }

    pub fn write_chr(&mut self, addr: u16, data: u8) {
        self.mapper.write_chr(self.rpg_rom.as_mut(), addr, data)
    }

    pub fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
}

impl Mirroring {
    pub fn to_adresses(&self) -> [u16; 4] {
        match self {
            Mirroring::Horizontal => [0x000, 0x000, 0x800, 0x800],
            Mirroring::Vertical => [0x000, 0x400, 0x000, 0x400],
            Mirroring::SingleScreen => [0x000, 0x000, 0x000, 0x000],
            Mirroring::FourScreen => [0x000, 0x400, 0x800, 0xc00],
        }
    }
}

#[allow(unused_variables)]
pub trait Mapper {
    fn read_rpg(&self, rpg: &[u8], addr: u16) -> u8;
    fn write_rpg(&mut self, rpg: &mut [u8], addr: u16, data: u8) {}

    fn read_chr(&self, chr: &[u8], addr: u16) -> u8;
    fn write_chr(&mut self, chr: &mut [u8], addr: u16, data: u8) {}
}
