use bit_field::BitField;
use std::cell::Cell;

/// PPU control register
#[derive(Debug, Default)]
pub struct PpuCtrl(u8);

impl PpuCtrl {
    pub fn set(&mut self, b: u8) {
        self.0 = b;
    }

    /// base nametable address
    pub fn nametable(&self) -> u16 {
        self.0.get_bits(..2) as u16
    }

    /// VRAM address increment per CPU read/write of PPUDATA
    pub fn addr_inc(&self) -> u16 {
        self.0.get_bit(2) as u16 * 31 + 1
    }

    /// sprite pattern table address for 8x8 sprites
    pub fn sp_pattern_table(&self) -> u16 {
        self.0.get_bit(3) as u16 * 0x1000
    }

    /// background pattern table address
    pub fn bg_pattern_table(&self) -> u16 {
        self.0.get_bit(4) as u16 * 0x1000
    }

    /// sprite size
    pub fn sp_size(&self) -> usize {
        (self.0.get_bit(5) as usize + 1) * 8
    }

    /// generate an NMI at the start of the vblank
    pub fn nmi_on(&self) -> bool {
        self.0.get_bit(7)
    }
}

/// PPU mask register
#[derive(Debug, Default)]
pub struct PpuMask(u8);

impl PpuMask {
    pub fn set(&mut self, b: u8) {
        self.0 = b;
    }

    /// gray scale display
    pub fn gray_scale(&self) -> bool {
        self.0.get_bit(0)
    }

    /// show background in leftmost 8 pixels of screen
    pub fn show_bg_left(&self) -> bool {
        self.0.get_bit(1)
    }

    /// show sprites in leftmost 8 pixels of screen
    pub fn show_sp_left(&self) -> bool {
        self.0.get_bit(2)
    }

    /// show background
    pub fn show_bg(&self) -> bool {
        self.0.get_bit(3)
    }

    /// show sprites
    pub fn show_sp(&self) -> bool {
        self.0.get_bit(4)
    }

    pub fn tint_red(&self) -> bool {
        self.0.get_bit(5)
    }

    pub fn tint_green(&self) -> bool {
        self.0.get_bit(6)
    }

    pub fn tint_blue(&self) -> bool {
        self.0.get_bit(7)
    }
}

/// PPU status register
#[derive(Debug, Default)]
pub struct PpuStatus(Cell<u8>);

impl PpuStatus {
    /// sprite overflow
    pub fn sp_overflow(&self) -> bool {
        self.0.get().get_bit(5)
    }

    /// sprite 0 hit
    pub fn sp0_hit(&self) -> bool {
        self.0.get().get_bit(6)
    }

    /// vblank
    pub fn vblank(&self) -> bool {
        self.0.get().get_bit(7)
    }

    pub fn set_lb(&self, b: u8) {
        self.0.set((self.0.get() & 0b1110_0000) | (b & 0b0001_1111));
    }

    pub fn set_sp_overflow(&self, b: bool) {
        self.0.set(*self.0.get().set_bit(5, b));
    }

    pub fn set_sp0_hit(&self, b: bool) {
        self.0.set(*self.0.get().set_bit(6, b));
    }

    pub fn set_vblank(&self, b: bool) {
        self.0.set(*self.0.get().set_bit(7, b));
    }

    pub fn get(&self) -> u8 {
        self.0.get()
    }
}

/// PPU vram address
///
/// from: https://wiki.nesdev.com/w/index.php?title=PPU_scrolling
/// fedcba98 76543210
///  yyyNNYY YYYXXXXX
#[derive(Debug, Default, Clone)]
pub struct VramAddr(Cell<u16>);

impl VramAddr {
    pub fn inc(&self, offset: u16) {
        self.0.set(self.0.get() + offset);
    }

    pub fn addr(&self) -> u16 {
        self.0.get().get_bits(0x00..0x0e)
    }

    pub fn coarse_x(&self) -> u16 {
        self.0.get().get_bits(0x00..0x05)
    }

    pub fn set_coarse_x(&self, b: u16) {
        self.0.set(*self.0.get().set_bits(0x00..0x05, b));
    }

    pub fn coarse_y(&self) -> u16 {
        self.0.get().get_bits(0x05..0x0a)
    }

    pub fn set_coarse_y(&self, b: u16) {
        self.0.set(*self.0.get().set_bits(0x05..0x0a, b));
    }

    pub fn nm(&self) -> u16 {
        self.0.get().get_bits(0x0a..0x0c)
    }

    pub fn set_nm(&self, b: u16) {
        self.0.set(*self.0.get().set_bits(0x0a..0x0c, b));
    }

    pub fn y(&self) -> u16 {
        self.0.get().get_bits(0x0c..0x0f)
    }

    pub fn set_y(&self, b: u16) {
        self.0.set(*self.0.get().set_bits(0x0c..0x0f, b));
    }

    pub fn set_bits<T: std::ops::RangeBounds<usize>>(&self, range: T, b: u16) {
        self.0.set(*self.0.get().set_bits(range, b));
    }
}
