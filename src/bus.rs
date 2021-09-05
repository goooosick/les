use crate::{Cartridge, Ppu};

const RAM_SIZE: usize = 0x0800;
const REG_SIZE: usize = 0x20;

pub struct Bus {
    ram: Box<[u8; RAM_SIZE]>,
    io_regs: Box<[u8; REG_SIZE]>,

    ppu: Ppu,
    cart: Cartridge,

    cycles: usize,
}

impl Bus {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            ram: Box::new([0u8; RAM_SIZE]),
            io_regs: Box::new([0u8; REG_SIZE]),

            ppu: Ppu::new(cart.mirroring()),
            cart,

            cycles: 0,
        }
    }

    pub fn tick(&mut self) {
        self.cycles += 1;
        for _ in 0..3 {
            self.ppu.tick();
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
            0x4014 => self.dma(data),
            0x4000..=0x401f => self.io_regs[addr as usize - 0x4000] = data,
            0x4020..=0xffff => self.cart.write(addr, data),
        }
    }

    pub fn inspect(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1fff => self.ram[addr as usize & 0x07ff],
            0x2000..=0x3fff => self.ppu.read(&self.cart, addr),
            0x4014 => 0x00,
            0x4000..=0x401f => self.io_regs[addr as usize - 0x4000],
            0x4020..=0xffff => self.cart.read(addr),
        }
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

impl Bus {
    fn dma(&mut self, data: u8) {
        self.tick();
        if self.cycles % 2 != 0 {
            self.tick();
        }

        let base = (data as u16) << 8;
        for i in 0x00..=0xff {
            let d = self.read(base + i);
            self.ppu.write_oam(i, d);
        }
    }
}
