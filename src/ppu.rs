use self::palettes::PALETTES;
use self::regs::*;
use crate::{Cartridge, Mirroring};
use std::cell::Cell;

mod palettes;
mod regs;

const OAM_SIZE: usize = 0x100;
const NAMETABLE_SIZE: usize = 0x1000;
const PALETTES_SIZE: usize = 0x20;

pub struct Ppu {
    nametables: Box<[u8; NAMETABLE_SIZE]>,
    palettes: Box<[u8; PALETTES_SIZE]>,
    oam: Box<[u8; OAM_SIZE]>,

    ctrl: PpuCtrl,
    mask: PpuMask,
    status: PpuStatus,
    oam_addr: usize,
    data_buf: Cell<u8>,

    v: VramAddr,
    t: VramAddr,
    x: usize,
    w: Cell<WriteLatch>,
    line: usize,
    dot: usize,

    nm_base_address: [u16; 4],
}

impl Ppu {
    pub fn new(mirroring: Mirroring) -> Self {
        Self {
            nametables: Box::new([0u8; NAMETABLE_SIZE]),
            palettes: Box::new([0u8; PALETTES_SIZE]),
            oam: Box::new([0u8; OAM_SIZE]),

            ctrl: PpuCtrl::default(),
            mask: PpuMask::default(),
            status: PpuStatus::default(),
            oam_addr: 0,
            data_buf: Cell::new(0),

            v: VramAddr::default(),
            t: VramAddr::default(),
            x: 0,
            w: Cell::new(WriteLatch::Step0),
            line: 0,
            dot: 0,

            nm_base_address: mirroring.to_adresses(),
        }
    }

    pub fn tick(&mut self) {
        self.dot += 1;
        if self.dot == 341 {
            self.line += 1;
            self.dot = 0;

            if self.line == 240 {
                self.status.set_vblank(true);
            }

            if self.line == 262 {
                self.line = 0;
                self.status.set_vblank(false);
            }
        }
    }
}

impl Ppu {
    pub fn read(&self, cart: &Cartridge, addr: u16) -> u8 {
        let addr = (addr - 0x2000) & 0x07;
        match addr {
            0x00 => panic!("ppu read: {:02x}", 0x00),
            0x01 => panic!("ppu read: {:02x}", 0x01),
            0x02 => {
                let b = self.status.get();
                self.status.set_vblank(false);
                self.w.set(WriteLatch::Step0);
                b
            }
            0x03 => panic!("ppu read: {:02x}", 0x03),
            0x04 => self.oam[self.oam_addr],
            0x05 => panic!("ppu read: {:02x}", 0x05),
            0x06 => panic!("ppu read: {:02x}", 0x06),
            0x07 => {
                let addr = self.v.addr();
                let data = self.data_buf.get();

                self.data_buf.set(self.read_vram(cart, addr));
                self.v.inc(self.ctrl.addr_inc());

                if addr < 0x3f00 {
                    data
                } else {
                    self.data_buf.get()
                }
            }

            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, cart: &mut Cartridge, addr: u16, data: u8) {
        let addr = (addr - 0x2000) & 0x07;
        match addr {
            0x00 => {
                self.ctrl.set(data);
                self.t.set_nm(self.ctrl.nametable());
            }
            0x01 => self.mask.set(data),
            0x02 => panic!("ppu write: {:02x}", 0x02),
            0x03 => self.oam_addr = data as usize,
            0x04 => {
                self.oam[self.oam_addr] = data;
                self.oam_addr = (self.oam_addr + 1) & 0xff;
            }
            0x05 => {
                if self.w.get() == WriteLatch::Step0 {
                    self.t.set_coarse_x((data >> 3) as u16);
                    self.x = (data & 0b0111) as usize;
                    self.w.set(WriteLatch::Step1);
                } else {
                    self.t.set_coarse_y((data >> 3) as u16);
                    self.t.set_y((data & 0b0111) as u16);
                    self.w.set(WriteLatch::Step0);
                }
            }
            0x06 => {
                if self.w.get() == WriteLatch::Step0 {
                    self.t.set_bits(0x08..0x0f, (data & 0b0011_1111) as u16);
                    self.w.set(WriteLatch::Step1);
                } else {
                    self.t.set_bits(0x00..0x08, data as u16);
                    self.v = self.t.clone();
                    self.w.set(WriteLatch::Step0);
                }
            }
            0x07 => {
                self.write_vram(cart, self.v.addr(), data);
                self.v.inc(self.ctrl.addr_inc());
            }

            _ => unreachable!(),
        }
    }

    pub fn write_oam(&mut self, addr: u16, data: u8) {
        self.oam[addr as usize] = data;
    }

    fn read_vram(&self, cart: &Cartridge, addr: u16) -> u8 {
        let addr = addr & 0x3fff;
        match addr {
            0x0000..=0x1fff => cart.read_chr(addr),
            0x2000..=0x3eff => self.nametables[self.nm_addr(addr)],
            0x3f00..=0x3fff => {
                let addr = (addr & 0x1f) as usize;
                match addr {
                    0x04 | 0x08 | 0x0c => self.palettes[0x00],
                    _ => self.palettes[addr],
                }
            }
            _ => unreachable!(),
        }
    }

    fn write_vram(&mut self, cart: &mut Cartridge, addr: u16, data: u8) {
        let addr = addr & 0x3fff;
        match addr {
            0x0000..=0x1fff => cart.write_chr(addr, data),
            0x2000..=0x3eff => self.nametables[self.nm_addr(addr)] = data,
            0x3f00..=0x3fff => {
                let data = data & 0x3f;
                let addr = (addr & 0x1f) as usize;
                match addr {
                    0x00 | 0x04 | 0x08 | 0x0c | 0x10 | 0x14 | 0x18 | 0x1c => {
                        self.palettes[addr & 0x0f + 0x00] = data;
                        self.palettes[addr & 0x0f + 0x10] = data;
                    }
                    _ => self.palettes[addr] = data,
                }
            }
            _ => unreachable!(),
        }
    }

    fn nm_addr(&self, addr: u16) -> usize {
        let n = (addr & 0xeff) >> 10;
        (self.nm_base_address[n as usize] + (addr & 0x3ff)) as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriteLatch {
    Step0,
    Step1,
}
