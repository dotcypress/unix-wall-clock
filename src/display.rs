use core::ops::Shr;

use hal::hal::blocking::spi;
use hal::hal::digital::v2::OutputPin;

pub struct DisplayController<SPI, LATCH> {
    spi: SPI,
    latch: LATCH,
    luma: u8,
    canvas: [u8; 80],
}

impl<SPI: spi::Write<u8>, LATCH: OutputPin> DisplayController<SPI, LATCH> {
    pub fn new(spi: SPI, latch: LATCH) -> Self {
        Self {
            spi,
            latch,
            luma: 0,
            canvas: [0; 80],
        }
    }

    pub fn update_segment(&mut self, idx: usize, luma: u8) {
        self.canvas[idx] = luma;
    }

    pub fn print(&mut self, pos: usize, symbol: u8, luma: u8) {
        for offset in 0..8 {
            let luma = if symbol.shr(offset) & 1 == 1 { luma } else { 0 };
            let idx = pos * 8 + 7 - offset;
            self.canvas[idx] = luma;
        }
    }

    pub fn render(&mut self) {
        let mut data = [0; 10];
        for (idx, chunk) in self.canvas.chunks(8).enumerate() {
            data[idx] = chunk
                .iter()
                .fold(0, |acc, luma| (acc << 1) | (*luma > self.luma) as u8);
        }
        self.luma = self.luma.wrapping_add(4);
        self.latch.set_low().ok();
        self.spi.write(&data).ok();
        self.latch.set_high().ok();
    }
}
