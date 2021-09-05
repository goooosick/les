use crate::Cartridge;

const RAM_SIZE: usize = 0x0800;
const REG_SIZE: usize = 0x2020;

pub struct Bus {
    ram: Box<[u8; RAM_SIZE]>,
    io_regs: Box<[u8; REG_SIZE]>,
    cart: Cartridge,

    cycles: usize,
}

impl Bus {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cycles: 0,

            ram: Box::new([0u8; RAM_SIZE]),
            io_regs: Box::new([0u8; REG_SIZE]),
            cart,
        }
    }

    pub fn tick(&mut self) {
        self.cycles += 1;
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        self.tick();
        self.inspect(addr)
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        self.tick();
        match addr {
            0x0000..=0x1fff => self.ram[addr as usize & 0x07ff] = data,
            0x2000..=0x401f => self.io_regs[addr as usize - 0x2000] = data,
            0x4020..=0xffff => self.cart.write(addr, data),
        }
    }

    pub fn inspect(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1fff => self.ram[addr as usize & 0x07ff],
            0x2000..=0x401f => self.io_regs[addr as usize - 0x2000],
            0x4020..=0xffff => self.cart.read(addr),
        }
    }

    pub fn cycles(&self) -> usize {
        self.cycles
    }
}
