const MEM_SIZE: usize = 0x1_0000;

pub struct Bus {
    cycles: usize,

    mem: Box<[u8; MEM_SIZE]>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            cycles: 7,

            mem: Box::new([0u8; MEM_SIZE]),
        }
    }

    pub fn tick(&mut self) {
        self.cycles += 1;
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        self.tick();
        self.mem[addr as usize]
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        self.tick();
        self.mem[addr as usize] = data;
    }

    pub fn inspect(&self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    pub fn load(&mut self, base: usize, data: &[u8]) {
        self.mem[base..=(base + data.len() - 1)].copy_from_slice(data);
    }

    pub fn cycles(&self) -> usize {
        self.cycles
    }
}
