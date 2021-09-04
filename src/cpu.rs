use crate::bus::Bus;
use addressing::{AddrMode, ADDR_MODES};
use status::Status;

mod addressing;
mod op_code;
mod status;

pub struct Cpu {
    a: u8,
    x: u8,
    y: u8,
    pc: u16,
    sp: u8,
    p: Status,

    op: u8,
    op_mode: AddrMode,
    op_address: u16,
    cross_page: bool,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            pc: 0,
            sp: 0xfd,
            p: 0x34u8.into(),

            op: 0xea,
            op_mode: AddrMode::IMP,
            op_address: 0,
            cross_page: false,
        }
    }

    pub fn exec(&mut self, bus: &mut Bus) {
        self.op = self.fetch_byte(bus);
        self.addressing(self.op, bus);

        let func = op_code::OP_FUNCS[self.op as usize];
        func(self, bus);

        let extra = op_code::OP_EXTRA_CYCLES[self.op as usize];
        for _ in 0..extra {
            bus.tick();
        }
    }

    pub fn set_pc(&mut self, addr: u16) {
        self.pc = addr;
    }

    pub fn dump(&self, bus: &Bus) {
        use op_code::OP_NAMES;

        print!("{:04X}  ", self.pc);

        let mut pc = self.pc;
        let op = bus.inspect(pc) as usize;
        let name = OP_NAMES[op];
        pc += 1;

        match ADDR_MODES[op] {
            AddrMode::IMP | AddrMode::ACC => {
                print!("{:02X}        {}", op, name);
            }
            AddrMode::IMM => {
                let op1 = bus.inspect(pc);
                print!("{:02X} {:02X}     {}", op, op1, name);
            }
            AddrMode::ZEP => {
                let op1 = bus.inspect(pc);
                print!("{:02X} {:02X}     {}", op, op1, name);
            }
            AddrMode::ZPX => {
                let op1 = bus.inspect(pc);
                print!("{:02X} {:02X}     {}", op, op1, name);
            }
            AddrMode::ZPY => {
                let op1 = bus.inspect(pc);
                print!("{:02X} {:02X}     {}", op, op1, name);
            }
            AddrMode::IZX => {
                let op1 = bus.inspect(pc);
                print!("{:02X} {:02X}     {}", op, op1, name);
            }
            AddrMode::IZY => {
                let op1 = bus.inspect(pc);
                print!("{:02X} {:02X}     {}", op, op1, name);
            }
            AddrMode::ABS => {
                let lb = bus.inspect(pc) as u16;
                let hb = bus.inspect(pc.wrapping_add(1)) as u16;
                print!("{:02X} {:02X} {:02X}  {}", op, lb, hb, name);
            }
            AddrMode::ABX => {
                let lb = bus.inspect(pc) as u16;
                let hb = bus.inspect(pc.wrapping_add(1)) as u16;
                print!("{:02X} {:02X} {:02X}  {}", op, lb, hb, name);
            }
            AddrMode::ABY => {
                let lb = bus.inspect(pc) as u16;
                let hb = bus.inspect(pc.wrapping_add(1)) as u16;
                print!("{:02X} {:02X} {:02X}  {}", op, lb, hb, name);
            }
            AddrMode::IND => {
                let lb = bus.inspect(pc) as u16;
                let hb = bus.inspect(pc.wrapping_add(1)) as u16;
                print!("{:02X} {:02X} {:02X}  {}", op, lb, hb, name);
            }
            AddrMode::REL => {
                let op1 = bus.inspect(pc);
                print!("{:02X} {:02X}     {}", op, op1, name);
            }
        }

        println!(
            " A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} CYC:{}",
            self.a,
            self.x,
            self.y,
            self.p.to_u8(),
            self.sp,
            bus.cycles()
        );
    }
}

impl Cpu {
    fn fetch_byte(&mut self, bus: &mut Bus) -> u8 {
        let b = bus.read(self.pc);
        self.pc += 1;
        b
    }

    fn fetch_word(&mut self, bus: &mut Bus) -> u16 {
        let d = self.read_word(self.pc, bus);
        self.pc += 2;
        d
    }

    fn read_word(&mut self, addr: u16, bus: &mut Bus) -> u16 {
        let lb = bus.read(addr) as u16;
        let hb = bus.read(addr + 1) as u16;
        (hb << 8) | lb
    }

    fn push_byte(&mut self, b: u8, bus: &mut Bus) {
        bus.write(0x100 + self.sp as u16, b);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn pop_byte(&mut self, bus: &mut Bus) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        bus.read(0x100 + self.sp as u16)
    }

    fn push_word(&mut self, b: u16, bus: &mut Bus) {
        self.push_byte((b >> 8) as u8, bus);
        self.push_byte(b as u8, bus);
    }

    fn pop_word(&mut self, bus: &mut Bus) -> u16 {
        let lb = self.pop_byte(bus) as u16;
        let hb = self.pop_byte(bus) as u16;
        (hb << 8) | lb
    }
}
