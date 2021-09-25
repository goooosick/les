use self::regs::*;
use crate::cart::{Cartridge, Mirroring};
use bit_field::BitField;
use std::{cell::Cell, ops::IndexMut};

pub use self::palettes::PALETTES;

mod palettes;
mod regs;

const OAM_SIZE: usize = 0x100;
const NAMETABLE_SIZE: usize = 0x1000;
const PALETTES_SIZE: usize = 0x20;
const BUF_SIZE: usize = 256 * 240 * 3;
const ACTIVE_OAM_SIZE: usize = 0x20;

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

    sp_n: usize,
    sp_count: usize,
    sec_oam: Box<[u8; ACTIVE_OAM_SIZE]>,
    sprites: Box<[SpriteState; 8]>,
    sp_zero: bool,

    buf: Box<[u8; BUF_SIZE]>,
    back_buf: Box<[u8; BUF_SIZE]>,
}

#[derive(Default, Debug)]
struct SpriteState {
    x: u8,
    tile_bits: ShiftReg,
    attr_bits: ShiftReg,
    is_sp_zero: bool,
    priority: u8,
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
}

impl Ppu {
    pub fn new() -> Self {
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

            // skip one dot on odd frame
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
        // on visible scanlines, output pixels
        if (0..240).contains(&self.line) && (1..257).contains(&self.dot) {
            let (mut bg_tile, mut bg_color) = (0, 0);
            let (mut sp_tile, mut sp_color) = (0, 0);
            let mut sp_priority = 0;
            let mut sp_zero = false;

            if self.mask.show_bg() {
                let bg_pal = self.rs.attr_bits.get(self.x);
                bg_tile = self.rs.tile_bits.get(self.x);
                bg_color = self.read_vram(cart, 0x3f00 + bg_pal * 4 + bg_tile);
            }

            if self.mask.show_sp() {
                for sp in self.rs.sprites.iter() {
                    if sp.x == 0 {
                        let sp_pal = sp.attr_bits.get(0);
                        let tile = sp.tile_bits.get(0);
                        if tile != 0 {
                            sp_tile = tile;
                            sp_color = self.read_vram(cart, 0x3f10 + sp_pal * 4 + sp_tile);
                            sp_priority = sp.priority;

                            if sp.is_sp_zero {
                                sp_zero = true;
                            }

                            break;
                        }
                    }
                }
            }

            if sp_zero && bg_tile != 0 && sp_tile != 0 {
                self.status.set_sp0_hit(true);
            }

            let pal_index = match (bg_tile != 0, sp_tile != 0, sp_priority) {
                (false, true, _) => sp_color,
                (true, true, 0) => sp_color,
                (_, _, _) => bg_color,
            };

            let index = (self.line * 256 + self.dot - 1) * 3;
            self.rs.buf[index..][..3].copy_from_slice(&PALETTES[pal_index as usize]);

            for sp in self.rs.sprites.iter_mut() {
                if sp.x > 0 {
                    sp.x -= 1;
                } else {
                    sp.tile_bits.shift();
                    sp.attr_bits.shift();
                }
            }
        }

        // on visible scanlines and pre-render scanline, fetch data
        if (0..240).contains(&self.line) || self.line == 261 {
            if self.mask.show_bg() {
                self.update_bg(cart);
            }

            if self.mask.show_sp() {
                self.update_sp(cart);
            }
        }
    }

    fn update_bg(&mut self, cart: &Cartridge) {
        if (1..257).contains(&self.dot) || (321..337).contains(&self.dot) {
            self.rs.attr_bits.shift();
            self.rs.tile_bits.shift();

            match (self.dot - 1) % 8 {
                1 => self.rs.nm_byte = self.read_vram(cart, self.v.tile_addr()),
                3 => self.rs.attr_byte = self.read_vram(cart, self.v.attr_addr()),
                5 => {}
                7 => {
                    let (b0, b1, a0, a1) = {
                        let attr = (self.rs.attr_byte
                            >> ((self.v.coarse_x() & 0b10) + (self.v.coarse_y() & 0b10) * 2))
                            & 0b11;
                        let chr_addr = self.ctrl.bg_pattern_table()
                            + self.v.y()
                            + self.rs.nm_byte as u16 * 0x10;
                        (
                            self.read_vram(cart, chr_addr),
                            self.read_vram(cart, chr_addr + 8),
                            attr.get_bit(0) as u8 * 0xff,
                            attr.get_bit(1) as u8 * 0xff,
                        )
                    };
                    self.rs.tile_bits.latch(b0, b1);
                    self.rs.attr_bits.latch(a0, a1);

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

        // update vertical
        if self.line == 261 && (280..305).contains(&self.dot) {
            self.v.copy_vy(&self.t);
        }
    }

    fn update_sp(&mut self, cart: &Cartridge) {
        // 1..=64 dots, clear secondary oam
        if self.dot == 64 && self.line != 261 {
            self.rs.sec_oam.fill(0xff);
            self.rs.sp_n = 0;
            self.rs.sp_count = 0;
            self.rs.sp_zero = false;
            self.status.set_sp_overflow(false);
        }

        // sprite evaluation
        if (65..257).contains(&self.dot) && self.line != 261 {
            if self.rs.sp_n < 64 {
                let addr0 = self.rs.sp_n * 4;
                let y = self.oam[addr0] as usize;
                if (y..(y + self.ctrl.sp_size())).contains(&self.line) {
                    if self.rs.sp_count < 8 {
                        let addr1 = self.rs.sp_count * 4;
                        self.rs.sec_oam[addr1 + 0] = self.oam[addr0 + 0];
                        self.rs.sec_oam[addr1 + 1] = self.oam[addr0 + 1];
                        self.rs.sec_oam[addr1 + 2] = self.oam[addr0 + 2];
                        self.rs.sec_oam[addr1 + 3] = self.oam[addr0 + 3];

                        self.rs.sp_count += 1;
                        if self.rs.sp_n == 0 {
                            self.rs.sp_zero = true;
                        }
                    } else {
                        self.status.set_sp_overflow(true);
                    }
                }

                self.rs.sp_n += 1;
            }
        }

        // fetch sprite data
        if (257..321).contains(&self.dot) {
            if (self.dot - 257) % 8 == 7 {
                let sp_n = (self.dot - 257) / 8;

                let addr = sp_n * 4;
                let sp_y = self.line as u16 - self.rs.sec_oam[addr + 0] as u16;
                let index = self.rs.sec_oam[addr + 1] as u16;
                let attr = self.rs.sec_oam[addr + 2];
                let sp_x = self.rs.sec_oam[addr + 3];

                self.rs.sprites[sp_n].x = sp_x;
                self.rs.sprites[sp_n].is_sp_zero = sp_n == 0 && self.rs.sp_zero;

                if sp_n < self.rs.sp_count {
                    // 76543210
                    // ||||||||
                    // ||||||++- palette of sprite
                    // |||+++--- unimplemented
                    // ||+------ priority (0: in front of background; 1: behind background)
                    // |+------- flip sprite horizontally
                    // +-------- flip sprite vertically

                    let (mut tile_b0, mut tile_b1, attr_b0, attr_b1) = {
                        let tile_y = (sp_y & 0x07) ^ (attr.get_bit(7) as u16 * 0x07);

                        let tile_addr = if self.ctrl.sp_size() == 8 {
                            self.ctrl.sp_pattern_table() + index * 0x10
                        } else {
                            let tile_offset = ((sp_y >= 8) ^ attr.get_bit(7)) as u16;
                            ((index & 0b01) * 0x1000) + ((index & 0xfe) + tile_offset) * 0x10
                        };
                        (
                            self.read_vram(cart, tile_addr + tile_y),
                            self.read_vram(cart, tile_addr + tile_y + 8),
                            attr.get_bit(0) as u8 * 0xff,
                            attr.get_bit(1) as u8 * 0xff,
                        )
                    };
                    if attr.get_bit(6) {
                        tile_b0 = tile_b0.reverse_bits();
                        tile_b1 = tile_b1.reverse_bits();
                    }

                    let sp = &mut self.rs.sprites[sp_n];
                    sp.tile_bits.load(tile_b0, tile_b1);
                    sp.attr_bits.load(attr_b0, attr_b1);
                    sp.priority = attr.get_bit(5) as u8;
                }
            }
        }
    }

    pub(crate) fn poll_nmi(&mut self) -> bool {
        std::mem::replace(&mut self.nmi, false)
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
            0x00 => 0x00,
            0x01 => 0x00,
            0x02 => {
                let b = self.status.get();
                self.status.set_vblank(false);
                self.w.set(WriteLatch::Step0);
                b
            }
            0x03 => 0x00,
            0x04 => self.oam[self.oam_addr],
            0x05 => 0x00,
            0x06 => 0x00,
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
            0x02 => {}
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
            0x2000..=0x3eff => self.nametables[cart.nm_addr(addr)],
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
            0x2000..=0x3eff => self.nametables[cart.nm_addr(addr)] = data,
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
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            nm_byte: 0,
            attr_byte: 0,
            tile_bits: Default::default(),
            attr_bits: Default::default(),

            sp_n: 0,
            sp_count: 0,
            sec_oam: Box::new([0xff; ACTIVE_OAM_SIZE]),
            sprites: Default::default(),
            sp_zero: false,

            buf: Box::new([0u8; BUF_SIZE]),
            back_buf: Box::new([0u8; BUF_SIZE]),
        }
    }
}

impl Ppu {
    fn reder_tile<T: IndexMut<usize, Output = u8>>(
        &self,
        cart: &Cartridge,
        buf: &mut [T],
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

                buf[index][0] = c[0];
                buf[index][1] = c[1];
                buf[index][2] = c[2];
                index += 1;

                p0 <<= 1;
                p1 <<= 1;
            }

            index += row_offset;
        }
    }

    pub fn render_pattern_table<T: IndexMut<usize, Output = u8>>(
        &self,
        cart: &Cartridge,
        buf: &mut [T],
        pal_index: usize,
    ) {
        let pal_index = pal_index as u16 * 4;

        let mut render_tile = |i, j, offset| {
            let plane_addr = ((i + offset) * 16 + j) * 0x10;

            let start_index = (i * 8) * 32 * 8 + (j + offset) * 8;
            let row_offset = 31 * 8;
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

    pub fn render_name_table<T: IndexMut<usize, Output = u8>>(
        &self,
        cart: &Cartridge,
        buf: &mut [T],
        nm_index: usize,
    ) {
        let chr_offset = self.ctrl.bg_pattern_table() as usize;

        for i in 0..30 {
            for j in 0..32 {
                let n = self.nametables[i * 32 + j + nm_index * 0x400] as usize;
                let plane_addr = n * 0x10 + chr_offset;

                let attr = self.nametables[nm_index * 0x400 + 30 * 32 + (i / 4) * 8 + j / 4];
                let pal_index = ((attr >> ((j & 0b10) + (i & 0b10) * 2)) & 0b11) as u16 * 4;

                let start_index = (i * 8) * 32 * 8 + j * 8;
                let row_offset = 31 * 8;
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

    pub fn render_palettes<T: IndexMut<usize, Output = u8>>(&self, buf: &mut [T]) {
        buf.chunks_exact_mut(16).enumerate().for_each(|(n, buf)| {
            let n = (n & 0x0f) | ((n & 0x100) >> 4);
            let c = &PALETTES[self.palettes[n] as usize];
            buf.iter_mut().for_each(|buf| {
                buf[0] = c[0];
                buf[1] = c[1];
                buf[2] = c[2];
            });
        });
    }

    pub fn render_sprites<T: IndexMut<usize, Output = u8>>(&self, cart: &Cartridge, buf: &mut [T]) {
        let n_row = 2;
        let n_col = 32;
        let row_offset = (n_col - 1) * 8;

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
                let start_index = (i * 8) * n_col * 8 + j * 8;

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

    pub fn render_display<T: IndexMut<usize, Output = u8>>(&self, buf: &mut [T]) {
        for i in 0..240 {
            for j in 0..256 {
                let index = i * 256 + j;
                let c = &self.rs.back_buf[(index * 3)..];
                buf[index][0] = c[0];
                buf[index][1] = c[1];
                buf[index][2] = c[2];
            }
        }
    }
}
