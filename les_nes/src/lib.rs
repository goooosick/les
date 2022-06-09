#![allow(clippy::identity_op)]

pub mod apu;
pub mod bus;
pub mod cart;
pub mod cpu;
pub mod ppu;

pub use apu::{Apu, Resampler};
pub use bus::{Bus, InputStates};
pub use cart::Cartridge;
pub use cpu::Cpu;
pub use ppu::Ppu;

/// NES NTSC
pub const MASTER_CLOCK: f32 = 2147_7272.0;

/// cpu frequency
pub const CPU_FREQUENCY: f32 = MASTER_CLOCK / 12.0;
