#[derive(Default, Clone, Copy)]
pub struct Status {
    pub n: bool,
    pub v: bool,
    pub b: bool,
    pub d: bool,
    pub i: bool,
    pub z: bool,
    pub c: bool,
}

impl Status {
    pub fn set_zn(&mut self, a: u8) {
        self.z = a == 0;
        self.n = (a & 0x80) != 0;
    }

    pub fn to_u8(self) -> u8 {
        0b0010_0000
            | ((self.n as u8) << 7)
            | ((self.v as u8) << 6)
            | ((self.b as u8) << 4)
            | ((self.d as u8) << 3)
            | ((self.i as u8) << 2)
            | ((self.z as u8) << 1)
            | ((self.c as u8) << 0)
    }
}

impl From<u8> for Status {
    fn from(b: u8) -> Self {
        Self {
            n: (b & (1 << 7)) != 0,
            v: (b & (1 << 6)) != 0,
            b: false,
            d: (b & (1 << 3)) != 0,
            i: (b & (1 << 2)) != 0,
            z: (b & (1 << 1)) != 0,
            c: (b & (1 << 0)) != 0,
        }
    }
}

impl std::fmt::Debug for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn b(b: bool, s: &'static str) -> &'static str {
            if b {
                s
            } else {
                "-"
            }
        }
        write!(
            f,
            "{}{}-{}{}{}{}{}",
            b(self.n, "N"),
            b(self.v, "V"),
            b(self.b, "B"),
            b(self.d, "D"),
            b(self.i, "I"),
            b(self.z, "Z"),
            b(self.c, "C"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status() {
        let mut s = Status::default();
        assert_eq!(s.to_u8(), 0b0010_0000);

        s.b = true;
        assert!(s.b);

        s.b = false;
        assert!(!s.b);

        let s: Status = 0b1111_1111u8.into();
        assert_eq!(s.to_u8(), 0b1110_1111);

        assert_eq!(format!("{:?}", s), "NV--DIZC".to_owned());
    }
}
