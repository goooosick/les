pub mod apu;
pub mod bus;
pub mod cart;
pub mod cpu;
pub mod ppu;

pub use apu::Apu;
pub use bus::{Bus, InputStates};
pub use cart::Cartridge;
pub use cpu::Cpu;
pub use ppu::Ppu;

/// NES NTSC: (341 * 262 - 0.5) * 4 * 60 = 2144_1960
pub const MASTER_CLOCK: usize = 2144_1960;

/// cpu frequency = 2144_1960 / 12 = 178_6830
pub const CPU_FREQUENCY: usize = MASTER_CLOCK / 12;

/// apu frame sequencer frequency
pub const APU_FRAME_FREQUENCY: usize = 240;
