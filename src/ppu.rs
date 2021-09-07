use self::palettes::PALETTES;
use self::regs::*;
use crate::{Cartridge, Mirroring};
use std::cell::Cell;

mod palettes;
mod regs;

const OAM_SIZE: usize = 0x100;
const NAMETABLE_SIZE: usize = 0x1000;
const PALETTES_SIZE: usize = 0x20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriteLatch {
    Step0,
    Step1,
}

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
    nmi: bool,

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
            nmi: false,

            nm_base_address: mirroring.to_adresses(),
        }
    }

    pub fn tick(&mut self) {
        self.dot += 1;
        if self.dot == 341 {
            self.line += 1;
            self.dot = 0;

            if self.line == 262 {
                self.line = 0;
                self.status.set_vblank(false);
            }
        }

        if self.line == 241 && self.dot == 1 {
            self.status.set_vblank(true);

            if self.ctrl.nmi_on() {
                self.nmi = true;
            }
        }
    }

    pub(crate) fn consume_nmi(&mut self) -> bool {
        let nmi = self.nmi;
        self.nmi = false;
        nmi
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

impl Ppu {
    pub fn render_pattern_table(&self, cart: &Cartridge, buf: &mut [u8], pal_index: usize) {
        use bit_field::BitField;

        let pal_index = pal_index as u16 * 4;

        let mut render_tile = |n, offset| {
            let plane_addr = n * 0x10;
            let n = n & 0xff;

            for x in 0..8 {
                let mut p0 = cart.read_chr((plane_addr + x) as u16);
                let mut p1 = cart.read_chr((plane_addr + x + 8) as u16);

                for y in 0..8 {
                    let b = (p0.get_bit(7) as u16) | ((p1.get_bit(7) as u16) << 1);
                    let pal_addr = pal_index + b + 0x3f00;
                    let c = &PALETTES[self.read_vram(cart, pal_addr) as usize];

                    let index = (((n / 16 * 8 + x) * 32 + n % 16 + offset) * 8 + y) * 3;
                    buf[index..][..3].copy_from_slice(c);

                    p0 <<= 1;
                    p1 <<= 1;
                }
            }
        };

        for i in 0..256usize {
            render_tile(i, 0);
            render_tile(i + 256, 16);
        }
    }

    pub fn render_name_table(&self, cart: &Cartridge, buf: &mut [u8], nm_index: usize) {
        use bit_field::BitField;

        let chr_offset = self.ctrl.bg_pattern_table() as usize;

        for i in 0..30 {
            for j in 0..32 {
                let n = self.nametables[i * 32 + j + nm_index * 0x400] as usize;
                let plane_addr = n * 0x10 + chr_offset;

                let attr = self.nametables[nm_index * 0x400 + 30 * 32 + (i / 4) * 8 + j / 4];
                let pal_index = ((attr >> ((j % 2) * 4) + (i % 2) * 2) & 0b11) as u16 * 4;

                for x in 0..8usize {
                    let mut p0 = cart.read_chr((plane_addr + x) as u16);
                    let mut p1 = cart.read_chr((plane_addr + x + 8) as u16);

                    for y in 0..8usize {
                        let b = (p0.get_bit(7) as u16) | ((p1.get_bit(7) as u16) << 1);
                        let pal_addr = pal_index + b + 0x3f00;
                        let c = &PALETTES[self.read_vram(cart, pal_addr) as usize];

                        let index = ((i * 8 + x) * 32 * 8 + j * 8 + y) * 3;
                        buf[index..][..3].copy_from_slice(c);

                        p0 <<= 1;
                        p1 <<= 1;
                    }
                }
            }
        }
    }
}
