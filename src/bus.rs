use self::dma::Dma;
use self::joystick::Joystick;
use crate::{Cartridge, Cpu, Ppu};

pub use joystick::InputStates;

mod dma;
mod joystick;

const RAM_SIZE: usize = 0x0800;
const REG_SIZE: usize = 0x20;

pub struct Bus {
    ram: Box<[u8; RAM_SIZE]>,
    io_regs: Box<[u8; REG_SIZE]>,

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

            ppu: Ppu::new(cart.mirroring()),
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
        }
    }

    pub(crate) fn tick(&mut self) {
        let Self { ppu, cart, .. } = self;

        self.cycles += 1;
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
            0x4016..=0x4017 => self.joystick.write(addr, data),
            0x4000..=0x401f => self.io_regs[addr as usize - 0x4000] = data,
            0x4020..=0xffff => self.cart.write(addr, data),
        }
    }

    pub fn inspect(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1fff => self.ram[addr as usize & 0x07ff],
            0x2000..=0x3fff => self.ppu.read(&self.cart, addr),
            0x4014 => 0x00,
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

    pub(crate) fn nmi(&mut self) -> bool {
        self.ppu.consume_nmi()
    }

    pub(crate) fn reset(&mut self) {
        self.ppu.reset();
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
}
