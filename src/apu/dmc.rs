#[derive(Debug)]
pub struct Dmc {}

impl Dmc {
    pub fn irq(&self) -> bool {
        todo!()
    }
}

impl super::Channel for Dmc {
    fn sample(&mut self) -> u8 {
        0
    }

    fn tick(&mut self) {}

    fn tick_len(&mut self) {}

    fn tick_eve(&mut self) {}

    fn write_reg0(&mut self, data: u8) {}

    fn write_reg1(&mut self, data: u8) {}

    fn write_reg2(&mut self, data: u8) {}

    fn write_reg3(&mut self, data: u8) {}

    fn set_enable(&mut self, enable: bool) {}

    fn enabled(&self) -> bool {
        false
    }
}
