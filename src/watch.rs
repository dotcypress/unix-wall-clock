use infrared::protocols::nec::NecCommand;

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

impl Default for Watch {
    fn default() -> Self {
        Self::new()
    }
}

impl Watch {
    pub fn new() -> Self {
        Self {
            brightness: 4,
            mode: Mode::UnixClock,
            ts: 0,
        }
    }

    pub fn set_utc_time(
        &mut self,
        year: u64,
        month: u64,
        day: u64,
        hour: u64,
        minutes: u64,
        seconds: u64,
    ) {
        let month_yday: [u64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
        let year_adj = year + 4800;
        let febs = year_adj - (if month <= 2 { 1 } else { 0 });
        let leap_days = 1 + (febs / 4) - (febs / 100) + (febs / 400);
        let days = 365 * year_adj + leap_days + month_yday[month as usize - 1] + day - 1 - 2472692;
        self.ts = days * 24 * 60 * 60 + hour * 60 * 60 + minutes * 60 + seconds;
    }

    pub fn ir_command(&mut self, cmd: NecCommand) {
        match cmd.cmd {
            7 | 70 => self.mode = Mode::DoomsdayClock,
            71 | 69 => self.mode = Mode::UnixClock,
            68 | 21 => self.brightness = 16,
            64 | 13 => self.brightness = self.brightness.saturating_sub(8).max(8),
            67 => self.brightness = self.brightness.saturating_add(8),
            _ => {}
        }
    }

    pub fn animate(&mut self, display: &mut crate::Display) {
        const DIGITS: [u8; 10] = [
            0b0111111, 0b0000110, 0b1011011, 0b1001111, 0b1100110, 0b1101101, 0b1111101, 0b0000111,
            0b1111111, 0b1101111,
        ];

        match self.mode {
            Mode::DoomsdayClock | Mode::UnixClock => {
                let mut val = match self.mode {
                    Mode::UnixClock => self.ts,
                    Mode::DoomsdayClock => i32::MAX as u64 - self.ts,
                };
                for pos in 0..10 {
                    let digit = val % 10;
                    display.print(9 - pos, DIGITS[digit as usize], self.brightness);
                    val /= 10;
                }
            }
        }
    }
}
