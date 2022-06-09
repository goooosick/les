mod blip_buf;

use self::blip_buf::Blip;

pub struct Resampler {
    blip: Blip,
    last_sample: f32,
    clocks: usize,
}

impl Resampler {
    pub fn new(size: usize) -> Self {
        Self {
            blip: Blip::new(size),
            last_sample: 0.0,
            clocks: 0,
        }
    }

    pub fn set_rates(&mut self, sample_rate: f64) {
        self.blip
            .set_rates(crate::CPU_FREQUENCY as f64, sample_rate);
    }

    pub(crate) fn add_sample(&mut self, s: f32) {
        if s != self.last_sample {
            let delta = (s - self.last_sample) * i16::MAX as f32;
            self.blip.add_delta(self.clocks, delta as i32);
            self.last_sample = s;
        }

        self.clocks += 1;
    }

    pub fn clocks_needed(&self, samples: usize) -> usize {
        self.blip.clocks_needed(samples)
    }

    pub fn end_frame(&mut self) {
        self.blip.end_frame(self.clocks);
        self.clocks = 0;
    }

    pub fn read_samples(&mut self, buf: &mut [i16]) -> usize {
        self.blip.read_samples(buf, buf.len(), false)
    }

    pub fn clear(&mut self) {
        self.clocks = 0;
        self.last_sample = 0.0;
        self.blip.clear();
    }

    pub fn avail(&self) -> usize {
        self.blip.samples_avail()
    }
}
