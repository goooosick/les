use super::Cpu;
use crate::Bus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum AddrMode {
    IMP,
    ACC,
    IMM,
    ZEP,
    ZPX,
    ZPY,
    IZX,
    IZY,
    ABS,
    ABX,
    ABY,
    IND,
    REL,
}

#[rustfmt::skip]
pub const ADDR_MODES: [AddrMode; 256] = {
    use AddrMode::*;
    //  00   01   02   03   04   05   06   07   08   09   0a   0b   0c   0d   0e   0f  
    [
        IMP, IZX, IMP, IZX, ZEP, ZEP, ZEP, ZEP, IMP, IMM, ACC, IMP, ABS, ABS, ABS, ABS, // 00
        REL, IZY, IMP, IZY, ZPX, ZPX, ZPX, ZPX, IMP, ABY, IMP, ABY, ABX, ABX, ABX, ABX, // 01
        ABS, IZX, IMP, IZX, ZEP, ZEP, ZEP, ZEP, IMP, IMM, ACC, IMP, ABS, ABS, ABS, ABS, // 02
        REL, IZY, IMP, IZY, ZPX, ZPX, ZPX, ZPX, IMP, ABY, IMP, ABY, ABX, ABX, ABX, ABX, // 03
        IMP, IZX, IMP, IZX, ZEP, ZEP, ZEP, ZEP, IMP, IMM, ACC, IMP, ABS, ABS, ABS, ABS, // 04
        REL, IZY, IMP, IZY, ZPX, ZPX, ZPX, ZPX, IMP, ABY, IMP, ABY, ABX, ABX, ABX, ABX, // 05
        IMP, IZX, IMP, IZX, ZEP, ZEP, ZEP, ZEP, IMP, IMM, ACC, IMP, IND, ABS, ABS, ABS, // 06
        REL, IZY, IMP, IZY, ZPX, ZPX, ZPX, ZPX, IMP, ABY, IMP, ABY, ABX, ABX, ABX, ABX, // 07
        IMM, IZX, IMM, IZX, ZEP, ZEP, ZEP, ZEP, IMP, IMM, IMP, IMP, ABS, ABS, ABS, ABS, // 08
        REL, IZY, IMP, IMP, ZPX, ZPX, ZPY, ZPY, IMP, ABY, IMP, IMP, IMP, ABX, IMP, IMP, // 09
        IMM, IZX, IMM, IZX, ZEP, ZEP, ZEP, ZEP, IMP, IMM, IMP, IMP, ABS, ABS, ABS, ABS, // 0a
        REL, IZY, IMP, IZY, ZPX, ZPX, ZPY, ZPY, IMP, ABY, IMP, IMP, ABX, ABX, ABY, ABY, // 0b
        IMM, IZX, IMM, IZX, ZEP, ZEP, ZEP, ZEP, IMP, IMM, IMP, IMP, ABS, ABS, ABS, ABS, // 0c
        REL, IZY, IMP, IZY, ZPX, ZPX, ZPX, ZPX, IMP, ABY, IMP, ABY, ABX, ABX, ABX, ABX, // 0d
        IMM, IZX, IMM, IZX, ZEP, ZEP, ZEP, ZEP, IMP, IMM, IMP, IMM, ABS, ABS, ABS, ABS, // 0e
        REL, IZY, IMP, IZY, ZPX, ZPX, ZPX, ZPX, IMP, ABY, IMP, ABY, ABX, ABX, ABX, ABX, // 0f
    ]
};

impl Cpu {
    pub(crate) fn addressing(&mut self, op: u8, bus: &mut Bus) {
        self.op_address = 0;
        self.op_mode = ADDR_MODES[op as usize];
        self.cross_page = false;

        match self.op_mode {
            AddrMode::IMP | AddrMode::ACC => {}
            AddrMode::IMM => {
                self.op_address = self.pc;
                self.pc += 1;
            }
            AddrMode::ZEP => {
                self.op_address = self.fetch_byte(bus) as u16;
            }
            AddrMode::ZPX => {
                self.op_address = self.fetch_byte(bus).wrapping_add(self.x) as u16;
            }
            AddrMode::ZPY => {
                self.op_address = self.fetch_byte(bus).wrapping_add(self.y) as u16;
            }
            AddrMode::IZX => {
                let base = self.fetch_byte(bus).wrapping_add(self.x);
                let lb = bus.read(base as u16) as u16;
                let hb = bus.read(base.wrapping_add(1) as u16) as u16;

                self.op_address = (hb << 8) | lb;
            }
            AddrMode::IZY => {
                let base = self.fetch_byte(bus);
                let lb = bus.read(base as u16) as u16;
                let hb = bus.read(base.wrapping_add(1) as u16) as u16;

                let base = (hb << 8) | lb;
                self.op_address = base.wrapping_add(self.y as u16);
                self.check_page(base, self.op_address);
            }
            AddrMode::ABS => {
                self.op_address = self.fetch_word(bus);
            }
            AddrMode::ABX => {
                let base = self.fetch_word(bus);
                self.op_address = base.wrapping_add(self.x as u16);
                self.check_page(base, self.op_address);
            }
            AddrMode::ABY => {
                let base = self.fetch_word(bus);
                self.op_address = base.wrapping_add(self.y as u16);
                self.check_page(base, self.op_address);
            }
            AddrMode::IND => {
                let base = self.fetch_word(bus);
                let lb = bus.read(base) as u16;
                let hb = bus.read((base & 0xff00) | ((base & 0x00ff) + 1) & 0x00ff) as u16;
                self.op_address = (hb << 8) | lb;
            }
            AddrMode::REL => {
                let rel = self.fetch_byte(bus);
                self.op_address = self.pc.wrapping_add(rel as i8 as i16 as u16);
                self.check_page(self.pc, self.op_address);
            }
        }
    }

    fn check_page(&mut self, addr1: u16, addr2: u16) {
        self.cross_page = (addr1 & 0xff00) != (addr2 & 0xff00);
    }
}
