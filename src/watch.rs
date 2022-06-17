use infrared::protocols::nec::NecCommand;

use crate::remote;

#[allow(dead_code)]
enum Mode {
    UnixClock,
    DoomsdayClock,
}
pub struct Watch {
    mode: Mode,
    brightness: u8,
    ts: u64,
}

impl Watch {
    pub fn new() -> Self {
        Self {
            brightness: 4,
            mode: Mode::UnixClock,
            ts: 1_654_839_000_000,
        }
    }

    pub fn tick(&mut self) {
        self.ts += 1_000;
    }

    pub fn ir_command(&mut self, cmd: NecCommand) {
        match cmd.cmd {
            remote::A => self.mode = Mode::UnixClock,
            remote::B => self.mode = Mode::DoomsdayClock,
            remote::UP => self.ts = self.ts.saturating_add(1_000),
            remote::DOWN => self.ts = self.ts.saturating_sub(1_000),
            remote::RIGTH => self.ts = self.ts.saturating_add(10_000),
            remote::LEFT => self.ts = self.ts.saturating_sub(10_000),
            remote::PLUS => self.brightness = self.brightness.saturating_add(8),
            remote::MINUS => self.brightness = self.brightness.saturating_sub(8),
            _ => {}
        }
        // (Mode::UnixClock, Button::Up) | (Mode::DoomsdayClock, Button::Up) => self.ts += 250,
        // (Mode::UnixClock, Button::Down) | (Mode::DoomsdayClock, Button::Down) => self.ts -= 250,
    }

    pub fn animate(&self, display: &mut crate::Display) {
        const DIGITS: [u8; 10] = [
            0b0111111, 0b0000110, 0b1011011, 0b1001111, 0b1100110, 0b1101101, 0b1111101, 0b0000111,
            0b1111111, 0b1101111,
        ];

        match self.mode {
            Mode::DoomsdayClock | Mode::UnixClock => {
                let mut num = match self.mode {
                    Mode::UnixClock => self.ts / 1_000,
                    Mode::DoomsdayClock => i32::MAX as u64 - self.ts / 1_000,
                };

                for pos in 0..10 {
                    let digit = num % 10;
                    display.print(9 - pos, DIGITS[digit as usize], self.brightness);
                    num = num / 10;
                }
            }
        }
    }
}
