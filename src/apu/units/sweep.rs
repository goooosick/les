use super::Divider;
use bit_field::BitField;

#[derive(Debug)]
pub struct Sweep {
    divider: Divider,
    enable: bool,
    neg: bool,
    shift: u8,
    shifter: usize,
    reload: bool,
    channel_offset: usize,
    muting: bool,
}

impl Sweep {
    pub fn new(channel_offset: usize) -> Self {
        Self {
            divider: Divider::new(),
            enable: false,
            neg: false,
            shift: 0,
            shifter: 0,
            reload: false,
            channel_offset,
            muting: false,
        }
    }

    pub fn tick(&mut self, timer: &mut Divider) {
        self.sweep(timer);

        if self.divider.count() == 0 {
            if self.enable && self.shift > 0 && !self.muting {
                timer.set_period(self.shifter);
            }
        }

        if self.divider.count() == 0 || self.reload {
            self.reload = false;
            self.divider.reset();
        } else {
            self.divider.tick();
        }
    }

    fn sweep(&mut self, timer: &mut Divider) {
        let period = timer.period();
        let delta = period >> self.shift;
        if self.neg {
            self.shifter = period.saturating_sub(delta + self.channel_offset);
        } else {
            self.shifter = period + delta;
        }

        self.muting = timer.period() < 8 || self.shifter > 0x7ff;
    }

    pub fn load(&mut self, data: u8) {
        self.enable = data.get_bit(7);
        self.divider.set_period(data.get_bits(4..7) as usize);
        self.neg = data.get_bit(3);
        self.shift = data.get_bits(0..3);

        self.reload = true;
    }

    pub fn muting(&self) -> bool {
        self.muting
    }
}
