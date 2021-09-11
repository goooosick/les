use std::cell::Cell;

#[derive(Debug, Default)]
pub struct Joystick {
    input0: Input,
    input1: Input,
    reading: bool,
}

impl Joystick {
    pub fn read(&self, addr: u16) -> u8 {
        if !self.reading {
            self.input0.load();
            self.input1.load();
        }

        match addr {
            0x4016 => self.input0.next() | 0x40,
            0x4017 => self.input1.next() | 0x40,
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x4016 => {
                self.reading = (data & 0b01) == 0;
                self.input0.load();
                self.input1.load();
            }
            0x4017 => {}
            _ => unreachable!(),
        }
    }

    pub fn set_input0(&mut self, states: InputStates) {
        self.input0.states = states;
    }

    pub fn set_input1(&mut self, states: InputStates) {
        self.input1.states = states;
    }
}

#[derive(Debug, Default)]
pub struct InputStates {
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl InputStates {
    fn to_u8(&self) -> u8 {
        ((!self.a as u8) << 0)
            | ((!self.b as u8) << 1)
            | ((!self.select as u8) << 2)
            | ((!self.start as u8) << 3)
            | ((!self.up as u8) << 4)
            | ((!self.down as u8) << 5)
            | ((!self.left as u8) << 6)
            | ((!self.right as u8) << 7)
    }
}

#[derive(Debug, Default)]
struct Input {
    states: InputStates,
    input: Cell<u8>,
}

impl Input {
    fn load(&self) {
        self.input.set(self.states.to_u8())
    }

    fn next(&self) -> u8 {
        let b = self.input.get();
        self.input.set(b >> 1);
        !b & 0b01
    }
}
