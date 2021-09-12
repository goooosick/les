use self::regs::*;
use crate::cart::{Cartridge, Mirroring};
use bit_field::BitField;
use std::cell::Cell;

pub use self::palettes::PALETTES;

mod palettes;
mod regs;

const OAM_SIZE: usize = 0x100;
const NAMETABLE_SIZE: usize = 0x1000;
const PALETTES_SIZE: usize = 0x20;
const BUF_SIZE: usize = 256 * 240 * 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriteLatch {
    Step0,
    Step1,
}

struct RenderState {
    nm_byte: u8,
    attr_byte: u8,
    tile_bits: ShiftReg,
    attr_bits: ShiftReg,

    buf: Box<[u8; BUF_SIZE]>,
    back_buf: Box<[u8; BUF_SIZE]>,
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
    frames: usize,
    line: usize,
    dot: usize,
    nmi: bool,
    rs: RenderState,

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
            frames: 0,
            line: 0,
            dot: 0,
            nmi: false,
            rs: Default::default(),

            nm_base_address: mirroring.to_adresses(),
        }
    }

    pub fn tick(&mut self, cart: &Cartridge) {
        self.update(cart);

        if self.line == 241 && self.dot == 1 {
            self.status.set_vblank(true);

            if self.ctrl.nmi_on() {
                self.nmi = true;
            }
        }

        if self.line == 261 {
            if self.dot == 1 {
                self.status.set_vblank(false);
                self.status.set_sp0_hit(false);

                std::mem::swap(&mut self.rs.buf, &mut self.rs.back_buf);
            }

            if self.dot == 339 {
                if self.mask.show_bg() && (self.frames % 2 != 0) {
                    self.dot = 340;
                }
            }
        }

        self.dot += 1;
        if self.dot == 341 {
            self.line += 1;
            self.dot = 0;

            if self.line == 262 {
                self.line = 0;
                self.frames += 1;
            }
        }
    }

    fn update(&mut self, cart: &Cartridge) {
        if self.mask.show_bg() {
            self.update_bg(cart);
        }
    }

    fn update_bg(&mut self, cart: &Cartridge) {
        if (0..240).contains(&self.line) || self.line == 261 {
            if (1..257).contains(&self.dot) || (321..337).contains(&self.dot) {
                let current_x = self.dot - 1;

                // render bg
                if current_x < 256 && self.line < 240 {
                    let bg_pal = self.rs.attr_bits.get(self.x);
                    let bg_color = self.rs.tile_bits.get(self.x);
                    let pal_index = self.read_vram(cart, 0x3f00 + bg_pal * 4 + bg_color);

                    let index = (self.line * 256 + current_x) * 3;
                    self.rs.buf[index..][..3].copy_from_slice(&PALETTES[pal_index as usize]);
                }

                self.rs.attr_bits.shift();
                self.rs.tile_bits.shift();

                // fetch data
                match current_x % 8 {
                    1 => self.rs.nm_byte = self.read_vram(cart, self.v.tile_addr()),
                    3 => self.rs.attr_byte = self.read_vram(cart, self.v.attr_addr()),
                    5 => {}
                    7 => {
                        let b = (self.rs.attr_byte
                            >> ((self.v.coarse_x() & 0b10) + (self.v.coarse_y() & 0b10) * 2))
                            & 0b11;
                        self.rs
                            .attr_bits
                            .load((b & 0b01) * 0xff, ((b & 0b10) >> 1) * 0xff);

                        let chr_addr = self.ctrl.bg_pattern_table()
                            + self.v.y()
                            + self.rs.nm_byte as u16 * 0x10;
                        let (b0, b1) = (
                            self.read_vram(cart, chr_addr),
                            self.read_vram(cart, chr_addr + 8),
                        );
                        self.rs.tile_bits.load(b0, b1);

                        // x increment
                        self.v.inc_coarse_x();
                    }
                    _ => {}
                }

                // y increment
                if self.dot == 256 {
                    self.v.inc_y();
                }
            }

            // update horizontal
            if self.dot == 257 {
                self.v.copy_vx(&self.t);
            }
        }

        // update vertical
        if self.line == 261 && (280..305).contains(&self.dot) {
            self.v.copy_vy(&self.t);
        }
    }

    pub(crate) fn consume_nmi(&mut self) -> bool {
        let nmi = self.nmi;
        self.nmi = false;
        nmi
    }

    pub(crate) fn reset(&mut self) {
        self.dot = 0;
        self.line = 0;
        self.frames = 0;
        self.rs = Default::default();
        self.v = VramAddr::default();
        self.t = VramAddr::default();
        self.x = 0;
        self.w = Cell::new(WriteLatch::Step0);
        self.nametables.fill(0);
        self.palettes.fill(0);
    }

    pub fn timing(&self) -> (usize, usize) {
        (self.line, self.dot)
    }

    pub fn display_buf(&self) -> &[u8] {
        self.rs.back_buf.as_ref()
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

impl Default for RenderState {
    fn default() -> Self {
        Self {
            nm_byte: 0,
            attr_byte: 0,
            tile_bits: Default::default(),
            attr_bits: Default::default(),

            buf: Box::new([0u8; BUF_SIZE]),
            back_buf: Box::new([0u8; BUF_SIZE]),
        }
    }
}

impl Ppu {
    fn reder_tile(
        &self,
        cart: &Cartridge,
        buf: &mut [u8],
        row_offset: usize,
        chr_addr: u16,
        color: impl Fn(u16) -> u16,
    ) {
        let mut index = 0;

        for x in 0..8 {
            let mut p0 = cart.read_chr(chr_addr + x);
            let mut p1 = cart.read_chr(chr_addr + x + 8);

            for _ in 0..8 {
                let b = (p0.get_bit(7) as u16) | ((p1.get_bit(7) as u16) << 1);
                let c = &PALETTES[self.read_vram(cart, color(b)) as usize];

                buf[index..][..3].copy_from_slice(c);
                index += 3;

                p0 <<= 1;
                p1 <<= 1;
            }

            index += row_offset;
        }
    }

    pub fn render_pattern_table(&self, cart: &Cartridge, buf: &mut [u8], pal_index: usize) {
        let pal_index = pal_index as u16 * 4;

        let mut render_tile = |i, j, offset| {
            let plane_addr = ((i + offset) * 16 + j) * 0x10;

            let start_index = ((i * 8) * 32 * 8 + (j + offset) * 8) * 3;
            let row_offset = 31 * 8 * 3;
            self.reder_tile(
                cart,
                &mut buf[start_index..],
                row_offset,
                plane_addr as u16,
                |b| pal_index + b + 0x3f00,
            );
        };

        for i in 0..16 {
            for j in 0..16 {
                render_tile(i, j, 0usize);
                render_tile(i, j, 16);
            }
        }
    }

    pub fn render_name_table(&self, cart: &Cartridge, buf: &mut [u8], nm_index: usize) {
        let chr_offset = self.ctrl.bg_pattern_table() as usize;

        for i in 0..30 {
            for j in 0..32 {
                let n = self.nametables[i * 32 + j + nm_index * 0x400] as usize;
                let plane_addr = n * 0x10 + chr_offset;

                let attr = self.nametables[nm_index * 0x400 + 30 * 32 + (i / 4) * 8 + j / 4];
                let pal_index = ((attr >> ((j & 0b10) + (i & 0b10) * 2)) & 0b11) as u16 * 4;

                let start_index = ((i * 8) * 32 * 8 + j * 8) * 3;
                let row_offset = 31 * 8 * 3;
                self.reder_tile(
                    cart,
                    &mut buf[start_index..],
                    row_offset,
                    plane_addr as u16,
                    |b| pal_index + b + 0x3f00,
                );
            }
        }
    }

    pub fn render_palettes(&self, buf: &mut [u8]) {
        buf.chunks_exact_mut(16 * 3)
            .enumerate()
            .for_each(|(n, buf)| {
                let n = (n & 0x0f) | ((n & 0x100) >> 4);
                let c = &PALETTES[self.palettes[n] as usize];
                buf.chunks_exact_mut(3).for_each(|buf| {
                    buf.copy_from_slice(c);
                });
            });
    }

    pub fn render_sprites(&self, cart: &Cartridge, buf: &mut [u8]) {
        let n_row = 2;
        let n_col = 32;
        let row_offset = (n_col - 1) * 8 * 3;

        for i in 0..n_row {
            for j in 0..n_col {
                let addr = (i * n_col + j) * 4;
                let index = self.oam[addr + 1] as u16;
                let attr = self.oam[addr + 2];

                let tile_index = if self.ctrl.sp_size() == 8 {
                    self.ctrl.sp_pattern_table() + index * 0x10
                } else {
                    ((index & 0b01) * 0x1000) + (index & 0xfe) * 0x10
                };
                let pal_index = attr.get_bits(0..2) as u16 * 4;
                let start_index = ((i * 8) * n_col * 8 + j * 8) * 3;

                self.reder_tile(cart, &mut buf[start_index..], row_offset, tile_index, |b| {
                    if b == 0 {
                        0x3f00
                    } else {
                        0x3f10 + pal_index + b
                    }
                });
            }
        }
    }
}
