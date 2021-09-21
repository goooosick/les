use bit_field::BitField;
use std::collections::VecDeque;

mod dmc;
mod noise;
mod pulse;
mod triangle;
mod units;

use dmc::Dmc;
use noise::Noise;
use pulse::Pulse;
use triangle::Triangle;
use units::*;

trait Channel {
    fn sample(&mut self) -> u8;

    fn tick(&mut self);
    fn tick_len(&mut self);
    fn tick_eve(&mut self);

    fn write_reg0(&mut self, data: u8);
    fn write_reg1(&mut self, data: u8);
    fn write_reg2(&mut self, data: u8);
    fn write_reg3(&mut self, data: u8);

    fn set_enable(&mut self, enable: bool);
    fn enabled(&self) -> bool;
}

pub struct Apu {
    frame: FrameCounter,
    pulse1: Pulse,
    pulse2: Pulse,
    triangle: Triangle,
    noise: Noise,
    dmc: Dmc,

    cycles: usize,
    samples: VecDeque<f32>,
}

impl Apu {
    pub fn new() -> Self {
        Self {
            frame: FrameCounter::new(),
            pulse1: Pulse::new(false),
            pulse2: Pulse::new(true),
            triangle: Triangle::new(),
            noise: Noise::new(),
            dmc: Dmc::new(),

            cycles: 0,
            samples: VecDeque::new(),
        }
    }

    pub fn tick(&mut self) {
        let step = self.frame.tick();
        self.frame_tick(step);

        self.cycles += 1;
        if self.cycles % 2 == 0 {
            self.pulse1.tick();
            self.pulse2.tick();
            self.noise.tick();
            self.dmc.tick();
        }
        self.triangle.tick();

        let pulse_out = PULSE_TABLE[(self.pulse1.sample() + self.pulse2.sample()) as usize];
        let tnd_out = TND_TABLE
            [(self.triangle.sample() * 3 + self.noise.sample() * 2 + self.dmc.sample()) as usize];
        self.samples.push_back(pulse_out + tnd_out);
    }

    fn frame_tick(&mut self, step: Step) {
        if step.contains(Step::LENGTH) {
            self.pulse1.tick_len();
            self.pulse2.tick_len();
            self.triangle.tick_len();
            self.noise.tick_len();
            self.dmc.tick_len();
        }
        if step.contains(Step::ENVELOPE) {
            self.pulse1.tick_eve();
            self.pulse2.tick_eve();
            self.triangle.tick_eve();
            self.noise.tick_eve();
            self.dmc.tick_eve();
        }
    }

    pub fn dmc_request(&mut self) -> Option<u16> {
        self.dmc.read_sample()
    }

    pub fn dmc_response(&mut self, data: u8) {
        self.dmc.write_sample(data);
    }

    pub fn reset(&mut self) {
        self.write(0x4015, 0x00);
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x4015 => {
                let mut data = 0;
                data.set_bit(7, self.dmc.irq());
                data.set_bit(6, self.frame.irq());
                data.set_bit(4, self.dmc.enabled());
                data.set_bit(3, self.noise.enabled());
                data.set_bit(2, self.triangle.enabled());
                data.set_bit(1, self.pulse2.enabled());
                data.set_bit(0, self.pulse1.enabled());
                data
            }
            _ => 0x00,
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x4000 => self.pulse1.write_reg0(data),
            0x4001 => self.pulse1.write_reg1(data),
            0x4002 => self.pulse1.write_reg2(data),
            0x4003 => self.pulse1.write_reg3(data),

            0x4004 => self.pulse2.write_reg0(data),
            0x4005 => self.pulse2.write_reg1(data),
            0x4006 => self.pulse2.write_reg2(data),
            0x4007 => self.pulse2.write_reg3(data),

            0x4008 => self.triangle.write_reg0(data),
            0x4009 => self.triangle.write_reg1(data),
            0x400a => self.triangle.write_reg2(data),
            0x400b => self.triangle.write_reg3(data),

            0x400c => self.noise.write_reg0(data),
            0x400d => self.noise.write_reg1(data),
            0x400e => self.noise.write_reg2(data),
            0x400f => self.noise.write_reg3(data),

            0x4010 => self.dmc.write_reg0(data),
            0x4011 => self.dmc.write_reg1(data),
            0x4012 => self.dmc.write_reg2(data),
            0x4013 => self.dmc.write_reg3(data),

            0x4015 => {
                self.dmc.set_enable(data.get_bit(4));
                self.noise.set_enable(data.get_bit(3));
                self.triangle.set_enable(data.get_bit(2));
                self.pulse2.set_enable(data.get_bit(1));
                self.pulse1.set_enable(data.get_bit(0));
            }
            0x4017 => {
                self.frame.load(data);
                if data.get_bit(7) {
                    self.frame_tick(Step::LENGTH | Step::ENVELOPE);
                }
            }
            _ => {}
        }
    }

    pub fn sample_len(&self) -> usize {
        self.samples.len()
    }

    pub fn samples(&mut self) -> &mut VecDeque<f32> {
        &mut self.samples
    }
}

lazy_static::lazy_static! {
    static ref PULSE_TABLE: [f32; 31] = {
        let mut table = [0.0f32; 31];
        for n in 1..31 {
            table[n] = 95.52 / (8128.0 / n as f32 + 100.0);
        }
        table
    };

    static ref TND_TABLE: [f32; 203] = {
        let mut table = [0.0f32; 203];
        for n in 1..203 {
            table[n] = 163.67 / (24329.0 / n as f32 + 100.0);
        }
        table
    };
}
