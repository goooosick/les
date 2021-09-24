#[derive(Debug, Default)]
pub struct Dma {
    dma_active: bool,
    dma_addr: u16,
    dma_step: u16,
    delay_ticks: u8,
}

impl Dma {
    pub fn reset(&mut self) {
        self.dma_active = false;
    }

    pub fn active(&self) -> bool {
        self.dma_active
    }

    pub fn start(&mut self, cycles: usize, data: u8) {
        self.dma_addr = (data as u16) << 8;
        self.dma_step = 0;
        self.dma_active = true;
        self.delay_ticks = 1 + (cycles % 2 != 0) as u8;
    }

    pub fn tick(&mut self) -> Option<u16> {
        if self.delay_ticks > 0 {
            self.delay_ticks -= 1;
            None
        } else {
            let r = Some(self.dma_addr + self.dma_step);

            self.dma_step += 1;
            if self.dma_step == 0x100 {
                self.dma_active = false;
            }

            r
        }
    }
}
