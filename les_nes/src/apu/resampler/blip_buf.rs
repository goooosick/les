#![allow(unused)]

///! port of https://code.google.com/archive/p/blip-buf/

pub const MAX_RATIO: u64 = 1 << 20;
pub const MAX_FRAME: u64 = 4000;

const PRE_SHIFT: u64 = 32;
const TIME_BITS: u64 = PRE_SHIFT + 20;
const TIME_UNIT: u64 = 1 << TIME_BITS;
const BASS_SHIFT: u64 = 9;
const PHASE_BITS: u64 = 5;
const PHASE_COUNT: usize = 1 << PHASE_BITS;
const DELTA_BITS: u64 = 15;
const DELTA_UNIT: u64 = 1 << DELTA_BITS;
const FRAC_BITS: u64 = TIME_BITS - PRE_SHIFT;

const END_FRAME_EXTRA: usize = 2;
const HALF_WIDTH: usize = 8;
const BUF_EXTRA: usize = HALF_WIDTH * 2 + END_FRAME_EXTRA;

pub struct Blip {
    factor: u64,
    offset: u64,
    avail: usize,
    size: usize,
    integrator: i32,
    buf: Box<[i32]>,
}

impl Blip {
    pub fn new(size: usize) -> Self {
        let factor = TIME_UNIT / MAX_RATIO;
        Self {
            factor,
            offset: factor / 2,
            avail: 0,
            size,
            integrator: 0,
            buf: vec![0i32; size + BUF_EXTRA].into_boxed_slice(),
        }
    }

    pub fn set_rates(&mut self, clock_rate: f64, sample_rate: f64) {
        let factor = TIME_UNIT as f64 * sample_rate / clock_rate;
        self.factor = factor.ceil() as _;
    }

    pub fn clear(&mut self) {
        self.offset = self.factor / 2;
        self.avail = 0;
        self.integrator = 0;
        self.buf.fill(0);
    }

    pub fn clocks_needed(&self, samples: usize) -> usize {
        debug_assert!(self.avail + samples <= self.size);

        let needed = TIME_UNIT * samples as u64;
        if needed < self.offset {
            return 0;
        }

        ((needed - self.offset + self.factor - 1) / self.factor) as usize
    }

    pub fn end_frame(&mut self, t: usize) {
        let off = t as u64 * self.factor + self.offset;
        self.avail += (off >> TIME_BITS) as usize;
        self.offset = off & (TIME_UNIT - 1);

        debug_assert!(self.avail <= self.size as _);
    }

    pub fn samples_avail(&self) -> usize {
        self.avail
    }

    fn remove_samples(&mut self, count: usize) {
        let remain = self.avail + BUF_EXTRA - count;
        self.avail -= count;

        self.buf.copy_within(count..count + remain, 0);
        self.buf[remain..].fill(0);
    }

    pub fn read_samples(&mut self, buf: &mut [i16], count: usize, stereo: bool) -> usize {
        let count = count.min(self.avail);

        if count > 0 {
            let step = if stereo { 2 } else { 1 };
            let mut sum = self.integrator;

            for (&sb, b) in self.buf[..count].iter().zip(buf.iter_mut().step_by(step)) {
                let s = sum >> DELTA_BITS;
                sum = sum.wrapping_add(sb);
                let s = s.clamp(i16::MIN as i32, i16::MAX as i32);

                *b = s as i16;

                sum -= s << (DELTA_BITS - BASS_SHIFT);
            }

            self.integrator = sum;
            self.remove_samples(count);
        }

        count
    }

    pub fn add_delta(&mut self, time: usize, delta: i32) {
        let fixed = (time as u64 * self.factor + self.offset) >> PRE_SHIFT;
        let out = &mut self.buf[(self.avail + (fixed >> FRAC_BITS) as usize)..];

        let phase_shift = FRAC_BITS - PHASE_BITS;
        let phase = (fixed >> phase_shift) & (PHASE_COUNT as u64 - 1);

        let interp = (fixed >> (phase_shift - DELTA_BITS)) & (DELTA_UNIT - 1);
        let delta2 = (delta * interp as i32) >> DELTA_BITS;
        let delta = delta - delta2;

        let in0 = &BL_STEP[phase as usize];
        let in1 = &BL_STEP[(phase + 1) as usize];
        let rev0 = &BL_STEP[PHASE_COUNT - phase as usize];
        let rev1 = &BL_STEP[PHASE_COUNT - phase as usize - 1];

        for i in 0..8 {
            out[i] += in0[i] * delta + in1[i] * delta2;
            out[8 + i] += rev0[7 - i] * delta + rev1[7 - i] * delta2;
        }
    }

    pub fn add_delta_fast(&mut self, time: usize, delta: i32) {
        let fixed = (time as u64 * self.factor + self.offset) >> PRE_SHIFT;
        let out = &mut self.buf[(self.avail + (fixed as usize >> FRAC_BITS))..];

        let interp = (fixed >> (FRAC_BITS - DELTA_BITS)) & (DELTA_UNIT - 1);
        let delta2 = delta * interp as i32;

        out[7] += delta * DELTA_UNIT as i32 - delta2;
        out[8] += delta2;
    }
}

const BL_STEP: [[i32; HALF_WIDTH]; PHASE_COUNT + 1] = [
    [43, -115, 350, -488, 1136, -914, 5861, 21022],
    [44, -118, 348, -473, 1076, -799, 5274, 21001],
    [45, -121, 344, -454, 1011, -677, 4706, 20936],
    [46, -122, 336, -431, 942, -549, 4156, 20829],
    [47, -123, 327, -404, 868, -418, 3629, 20679],
    [47, -122, 316, -375, 792, -285, 3124, 20488],
    [47, -120, 303, -344, 714, -151, 2644, 20256],
    [46, -117, 289, -310, 634, -17, 2188, 19985],
    [46, -114, 273, -275, 553, 117, 1758, 19675],
    [44, -108, 255, -237, 471, 247, 1356, 19327],
    [43, -103, 237, -199, 390, 373, 981, 18944],
    [42, -98, 218, -160, 310, 495, 633, 18527],
    [40, -91, 198, -121, 231, 611, 314, 18078],
    [38, -84, 178, -81, 153, 722, 22, 17599],
    [36, -76, 157, -43, 80, 824, -241, 17092],
    [34, -68, 135, -3, 8, 919, -476, 16558],
    [32, -61, 115, 34, -60, 1006, -683, 16001],
    [29, -52, 94, 70, -123, 1083, -862, 15422],
    [27, -44, 73, 106, -184, 1152, -1015, 14824],
    [25, -36, 53, 139, -239, 1211, -1142, 14210],
    [22, -27, 34, 170, -290, 1261, -1244, 13582],
    [20, -20, 16, 199, -335, 1301, -1322, 12942],
    [18, -12, -3, 226, -375, 1331, -1376, 12293],
    [15, -4, -19, 250, -410, 1351, -1408, 11638],
    [13, 3, -35, 272, -439, 1361, -1419, 10979],
    [11, 9, -49, 292, -464, 1362, -1410, 10319],
    [9, 16, -63, 309, -483, 1354, -1383, 9660],
    [7, 22, -75, 322, -496, 1337, -1339, 9005],
    [6, 26, -85, 333, -504, 1312, -1280, 8355],
    [4, 31, -94, 341, -507, 1278, -1205, 7713],
    [3, 35, -102, 347, -506, 1238, -1119, 7082],
    [1, 40, -110, 350, -499, 1190, -1021, 6464],
    [0, 43, -115, 350, -488, 1136, -914, 5861],
];
