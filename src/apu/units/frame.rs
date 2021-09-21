use super::Divider;
use bit_field::BitField;
use std::cell::Cell;

// mode 0:    mode 1:       function
// ---------  -----------  -----------------------------
//  - - - f    - - - - -    IRQ (if bit 6 is clear)
//  - l - l    - l - - l    Length counter and sweep
//  e e e e    e e e - e    Envelope and linear counter

#[derive(Debug, PartialEq, Eq)]
enum Mode {
    Step4,
    Step5,
}

bitflags::bitflags! {
    pub struct Step: u8 {
        const LENGTH   = 0b01;
        const ENVELOPE = 0b10;
    }
}

#[derive(Debug)]
pub struct FrameCounter {
    divider: Divider,
    step: usize,
    mode: Mode,
    irq_on: bool,
    irq_level: Cell<bool>,
}

impl FrameCounter {
    pub fn new() -> Self {
        Self {
            divider: Divider::with_period(crate::CPU_FREQUENCY / crate::APU_FRAME_FREQUENCY),
            step: 0,
            mode: Mode::Step4,
            irq_on: false,
            irq_level: Cell::default(),
        }
    }

    pub fn tick(&mut self) -> Step {
        let mut step = Step::empty();

        if self.divider.tick() {
            match self.mode {
                Mode::Step4 => {
                    self.step = (self.step + 1) % 4;
                    step.set(Step::LENGTH, self.step == 1 || self.step == 3);
                    step.set(Step::ENVELOPE, true);
                    if self.irq_on && self.step == 3 {
                        self.irq_level.set(true);
                    }
                }
                Mode::Step5 => {
                    self.step = (self.step + 1) % 5;
                    step.set(Step::LENGTH, self.step == 1 || self.step == 4);
                    step.set(Step::ENVELOPE, self.step != 3);
                }
            }
        }

        step
    }

    pub fn load(&mut self, data: u8) {
        self.step = 0;
        self.divider.reset();
        self.mode = if data.get_bit(7) {
            Mode::Step5
        } else {
            Mode::Step4
        };
        self.irq_on = !data.get_bit(6);
    }

    pub fn irq(&self) -> bool {
        self.irq_level.replace(false)
    }
}
