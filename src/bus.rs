use std::collections::VecDeque;

use self::dma::Dma;
use self::joystick::Joystick;
use crate::{cpu::Interrupt, Apu, Cartridge, Cpu, Ppu};

pub use joystick::InputStates;

mod dma;
mod joystick;

const RAM_SIZE: usize = 0x0800;
const REG_SIZE: usize = 0x20;

pub struct Bus {
    ram: Box<[u8; RAM_SIZE]>,
    io_regs: Box<[u8; REG_SIZE]>,

    apu: Apu,
    ppu: Ppu,
    cart: Cartridge,
    joystick: Joystick,
    dma: Dma,

    cycles: usize,
}

impl Bus {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            ram: Box::new([0u8; RAM_SIZE]),
            io_regs: Box::new([0u8; REG_SIZE]),

            apu: Apu::new(),
            ppu: Ppu::new(),
            cart,
            joystick: Default::default(),
            dma: Default::default(),

            cycles: 0,
        }
    }

    pub fn exec(&mut self, cpu: &mut Cpu) {
        if self.dma.active() {
            if let Some(addr) = self.dma.tick() {
                let data = self.read(addr);
                self.write(0x2004, data);
            } else {
                self.tick();
            }
        } else {
            cpu.exec(self);

            if self.ppu.poll_nmi() {
                cpu.serve_interrupt(Interrupt::NMI, self);
            } else if !cpu.interrupt_disabled() {
                if self.apu.poll_irq() || self.cart.poll_irq() {
                    cpu.serve_interrupt(Interrupt::IRQ, self);
                }
            }
        }
    }

    pub(crate) fn tick(&mut self) {
        self.cycles += 1;

        self.apu.tick();
        if let Some(addr) = self.apu.dmc_request() {
            let data = self.inspect(addr);
            self.tick_ppu();
            self.apu.dmc_response(data);
            self.tick_ppu();
        } else {
            self.tick_ppu();
        }
    }

    fn tick_ppu(&mut self) {
        let Self { ppu, cart, .. } = self;
        for _ in 0..3 {
            ppu.tick(cart);
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        self.tick();
        self.inspect(addr)
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        self.tick();
        match addr {
            0x0000..=0x1fff => self.ram[addr as usize & 0x07ff] = data,
            0x2000..=0x3fff => self.ppu.write(&mut self.cart, addr, data),
            0x4014 => self.dma.start(self.cycles, data),
            0x4000..=0x4013 => self.apu.write(addr, data),
            0x4015 | 0x4017 => self.apu.write(addr, data),
            0x4016 => self.joystick.write(addr, data),
            0x4000..=0x401f => self.io_regs[addr as usize - 0x4000] = data,
            0x4020..=0xffff => self.cart.write(addr, data),
        }
    }

    pub fn inspect(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1fff => self.ram[addr as usize & 0x07ff],
            0x2000..=0x3fff => self.ppu.read(&self.cart, addr),
            0x4014 => 0x00,
            0x4000..=0x4015 => self.apu.read(addr),
            0x4016..=0x4017 => self.joystick.read(addr),
            0x4000..=0x401f => self.io_regs[addr as usize - 0x4000],
            0x4020..=0xffff => self.cart.read(addr),
        }
    }

    pub fn set_input0(&mut self, states: InputStates) {
        self.joystick.set_input0(states);
    }

    pub fn set_input1(&mut self, states: InputStates) {
        self.joystick.set_input1(states);
    }

    pub fn reset(&mut self, cpu: &mut Cpu) {
        self.ppu.reset();
        self.apu.reset();
        self.dma.reset();
        self.cycles = 0;

        cpu.serve_interrupt(Interrupt::RESET, self);
    }

    pub fn cycles(&self) -> usize {
        self.cycles
    }

    pub fn ppu(&self) -> &Ppu {
        &self.ppu
    }

    pub fn cart(&self) -> &Cartridge {
        &self.cart
    }

    pub fn apu(&self) -> &Apu {
        &self.apu
    }

    pub fn audio_samples(&mut self) -> &mut VecDeque<f32> {
        self.apu.samples()
    }

    /// override channel state (pulse1, pulse2, triangle, noise, dmc)
    pub fn set_audio_control(&mut self, states: &[bool; 5]) {
        self.apu.set_channels(states);
    }

    pub fn load_cart(&mut self, cart: Cartridge) {
        self.cart = cart;
    }
}
