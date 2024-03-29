use bit_field::BitField;

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
    pub fn _gray_scale(&self) -> bool {
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

    pub fn _tint_red(&self) -> bool {
        self.0.get_bit(5)
    }

    pub fn _tint_green(&self) -> bool {
        self.0.get_bit(6)
    }

    pub fn _tint_blue(&self) -> bool {
        self.0.get_bit(7)
    }
}

/// PPU status register
#[derive(Debug, Default)]
pub struct PpuStatus(u8);

impl PpuStatus {
    pub fn set_sp_overflow(&mut self, b: bool) {
        self.0.set_bit(5, b);
    }

    pub fn set_sp0_hit(&mut self, b: bool) {
        self.0.set_bit(6, b);
    }

    pub fn set_vblank(&mut self, b: bool) {
        self.0.set_bit(7, b);
    }

    pub fn get(&self) -> u8 {
        self.0
    }
}

// from: https://wiki.nesdev.com/w/index.php?title=PPU_scrolling
// fedcba98 76543210
//  yyyNNYY YYYXXXXX
const VX_MASK: u16 = 0b0000_0100_0001_1111;
const VY_MASK: u16 = 0b0111_1011_1110_0000;

/// PPU vram address
#[derive(Debug, Default, Clone)]
pub struct VramAddr(u16);

impl VramAddr {
    pub fn addr(&self) -> u16 {
        self.0.get_bits(0x00..0x0e)
    }

    pub fn tile_addr(&self) -> u16 {
        0x2000 | (self.0 & 0x0fff)
    }

    pub fn attr_addr(&self) -> u16 {
        let v = self.0;
        0x23C0 | (v & 0x0c00) | ((v >> 4) & 0x38) | ((v >> 2) & 0x07)
    }

    pub fn inc(&mut self, offset: u16) {
        self.0 += offset;
    }

    pub fn inc_coarse_x(&mut self) {
        let cx = self.coarse_x();
        if cx == 31 {
            self.set_coarse_x(0);
            self.switch_nm(0b01); // switch horizontal nametable
        } else {
            self.set_coarse_x(cx + 1);
        }
    }

    pub fn inc_y(&mut self) {
        let y = self.y();
        if y < 7 {
            self.set_y(y + 1);
        } else {
            self.set_y(0);
            let cy = self.coarse_y();
            if cy == 29 {
                self.set_coarse_y(0);
                self.switch_nm(0b10); // switch vertical nametable
            } else if cy == 31 {
                self.set_coarse_y(0);
            } else {
                self.set_coarse_y(cy + 1);
            }
        }
    }

    pub fn coarse_x(&self) -> u16 {
        self.0.get_bits(0x00..0x05)
    }

    pub fn set_coarse_x(&mut self, b: u16) {
        self.0.set_bits(0x00..0x05, b);
    }

    pub fn coarse_y(&self) -> u16 {
        self.0.get_bits(0x05..0x0a)
    }

    pub fn set_coarse_y(&mut self, b: u16) {
        self.0.set_bits(0x05..0x0a, b);
    }

    pub fn nm(&self) -> u16 {
        self.0.get_bits(0x0a..0x0c)
    }

    pub fn set_nm(&mut self, b: u16) {
        self.0.set_bits(0x0a..0x0c, b);
    }

    pub fn switch_nm(&mut self, b: u16) {
        self.set_nm(self.nm() ^ b);
    }

    pub fn y(&self) -> u16 {
        self.0.get_bits(0x0c..0x0f)
    }

    pub fn set_y(&mut self, b: u16) {
        self.0.set_bits(0x0c..0x0f, b);
    }

    pub fn set_bits<T: std::ops::RangeBounds<usize>>(&mut self, range: T, b: u16) {
        self.0.set_bits(range, b);
    }

    pub fn copy_vx(&mut self, other: &VramAddr) {
        let v0 = self.0;
        let v1 = other.0;
        self.0 = (v0 & !VX_MASK) | (v1 & VX_MASK);
    }

    pub fn copy_vy(&mut self, other: &VramAddr) {
        let v0 = self.0;
        let v1 = other.0;
        self.0 = (v0 & !VY_MASK) | (v1 & VY_MASK);
    }
}

#[derive(Debug, Default)]
pub struct ShiftReg(u16, u16);

impl ShiftReg {
    pub fn get(&self, x: usize) -> u16 {
        self.0.get_bit(x) as u16 | ((self.1.get_bit(x) as u16) << 1)
    }

    pub fn shift(&mut self) {
        self.0 >>= 1;
        self.1 >>= 1;
    }

    pub fn latch(&mut self, b0: u8, b1: u8) {
        self.0.set_bits(8..16, b0.reverse_bits() as u16);
        self.1.set_bits(8..16, b1.reverse_bits() as u16);
    }

    pub fn load(&mut self, b0: u8, b1: u8) {
        self.0.set_bits(0..8, b0.reverse_bits() as u16);
        self.1.set_bits(0..8, b1.reverse_bits() as u16);
    }
}
