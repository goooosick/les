use bit_field::BitField;

mod mapper000;
mod mapper001;
mod mapper002;
mod mapper003;
mod mapper004;

const EXPANSION_ROM_SIZE: usize = 0x1fe0;
const PRG_RAM_SIZE: usize = 0x2000;
const CHR_RAM_SIZE: usize = 0x2000;

const MIRRORING_MAP: [[usize; 4]; 5] = [
    [0x000, 0x000, 0x400, 0x400], // Horizontal
    [0x000, 0x400, 0x000, 0x400], // Vertical
    [0x000, 0x000, 0x000, 0x000], // SingleScreen0
    [0x400, 0x400, 0x400, 0x400], // SingleScreen1
    [0x000, 0x400, 0x800, 0xc00], // FourScreen
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Mirroring {
    Horizontal = 0,
    Vertical = 1,
    SingleScreen0 = 2,
    SingleScreen1 = 3,
    FourScreen = 4,
}

pub struct Cartridge {
    expansion: Box<[u8; EXPANSION_ROM_SIZE]>,
    prg_ram: Box<[u8; PRG_RAM_SIZE]>,
    prg_rom: Vec<u8>,
    chr_ram: Box<[u8; CHR_RAM_SIZE]>,
    chr_rom: Vec<u8>,

    mapper: Box<dyn Mapper + Send + Sync>,
}

impl Cartridge {
    pub fn empty() -> Self {
        Cartridge {
            expansion: Box::new([0u8; EXPANSION_ROM_SIZE]),
            prg_ram: Box::new([0u8; PRG_RAM_SIZE]),
            prg_rom: Vec::new(),
            chr_ram: Box::new([0u8; CHR_RAM_SIZE]),
            chr_rom: Vec::new(),

            mapper: Box::new(NullMapper),
        }
    }

    pub fn load(data: &[u8]) -> Option<Self> {
        if data[..4] != [b'N', b'E', b'S', 0x1a] {
            return None;
        }
        if data[7] & 0b1100 == 0b1100 {
            panic!("iNes 2.0");
        }

        let f6 = data[6];
        let _ram = f6.get_bit(1);
        let trainer = f6.get_bit(2);
        let mirroring = if f6.get_bit(3) {
            Mirroring::FourScreen
        } else if f6.get_bit(0) {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };

        let mapper_type = (data[7] & 0xf0) | (f6 >> 4);

        let offset = 0x10 + (trainer as usize) * 0x200;
        let prg_banks = data[4] as usize;
        let prg_rom = data[offset..][..(prg_banks * 0x4000)].to_vec();

        let offset = offset + prg_rom.len();
        let chr_banks = data[5] as usize;
        let chr_len = chr_banks * 0x2000;

        let mut chr_rom = vec![0u8; chr_banks * 0x2000];
        chr_rom[..chr_len].copy_from_slice(&data[offset..][..chr_len]);

        println!("MAPPER: {:03}", mapper_type);
        println!("PRG ROM: {} * 16KB", prg_banks);
        println!("CHR ROM: {} * 8KB", chr_banks);
        println!("MIRRORING: {:?}", mirroring);

        Some(Self {
            expansion: Box::new([0u8; EXPANSION_ROM_SIZE]),
            prg_ram: Box::new([0u8; PRG_RAM_SIZE]),
            prg_rom,
            chr_ram: Box::new([0u8; CHR_RAM_SIZE]),
            chr_rom,

            mapper: match mapper_type {
                0 => Box::new(mapper000::Mapper000::new(mirroring, prg_banks)),
                1 => Box::new(mapper001::Mapper001::new(mirroring, prg_banks)),
                2 | 66 => Box::new(mapper002::Mapper002::new(mirroring, prg_banks)),
                3 => Box::new(mapper003::Mapper003::new(mirroring, prg_banks, chr_banks)),
                4 => Box::new(mapper004::Mapper004::new(mirroring, prg_banks)),
                _ => unimplemented!("unimplemented mapper type: {}", mapper_type),
            },
        })
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x4020..=0x5fff => self.expansion[addr as usize - 0x4020],
            0x6000..=0x7fff => self.prg_ram[addr as usize - 0x6000],
            0x8000..=0xffff => self.mapper.read_prg(self.prg_rom.as_ref(), addr),
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x4020..=0x5fff => self.expansion[addr as usize - 0x4020] = data,
            0x6000..=0x7fff => self.prg_ram[addr as usize - 0x6000] = data,
            0x8000..=0xffff => self.mapper.write_prg(self.prg_rom.as_mut(), addr, data),
            _ => unreachable!(),
        }
    }

    pub fn read_chr(&self, addr: u16) -> u8 {
        if !self.chr_rom.is_empty() {
            self.mapper.read_chr(self.chr_rom.as_ref(), addr)
        } else {
            self.chr_ram[addr as usize]
        }
    }

    pub fn write_chr(&mut self, addr: u16, data: u8) {
        self.mapper.write_chr(self.chr_ram.as_mut(), addr, data)
    }

    pub fn nm_addr(&self, addr: u16) -> usize {
        let n = (addr as usize & 0xeff) >> 10;
        let addr = addr as usize & 0x3ff;
        MIRRORING_MAP[self.mapper.mirroring() as usize][n] + addr
    }

    pub(crate) fn update_scanline(&mut self) {
        self.mapper.update_scanline();
    }

    pub(crate) fn poll_irq(&mut self) -> bool {
        self.mapper.poll_irq()
    }
}

#[allow(unused_variables)]
pub trait Mapper {
    fn read_prg(&self, prg: &[u8], addr: u16) -> u8;
    fn write_prg(&mut self, prg: &mut [u8], addr: u16, data: u8) {}

    fn read_chr(&self, chr: &[u8], addr: u16) -> u8;
    fn write_chr(&mut self, chr: &mut [u8], addr: u16, data: u8) {
        chr[addr as usize] = data;
    }

    fn update_scanline(&mut self) {}
    fn poll_irq(&mut self) -> bool {
        false
    }

    fn mirroring(&self) -> Mirroring;
}

struct NullMapper;

impl Mapper for NullMapper {
    fn read_prg(&self, _: &[u8], addr: u16) -> u8 {
        // an infinite loop program
        match addr {
            0xfffc => 0x00,
            0xfffd => 0xff,
            0xff00 => 0x4c,
            0xff01 => 0x00,
            0xff02 => 0xff,
            _ => unreachable!(),
        }
    }

    fn read_chr(&self, _: &[u8], _: u16) -> u8 {
        0x00
    }

    fn mirroring(&self) -> Mirroring {
        Mirroring::FourScreen
    }
}
