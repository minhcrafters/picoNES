use super::{LENGTH_TABLE, channel::Channel};

const NOISE_PERIOD_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

#[derive(Debug)]
pub struct NoiseChannel {
    enabled: bool,
    length_counter: u8,
    length_halt: bool,
    constant_volume: bool,
    envelope_period: u8,
    envelope_divider: u8,
    envelope_decay_level: u8,
    envelope_start: bool,
    mode_short: bool,
    shift_register: u16,
    timer_period: u16,
    timer_value: u16,
}

impl NoiseChannel {
    pub fn new() -> Self {
        Self {
            enabled: false,
            length_counter: 0,
            length_halt: false,
            constant_volume: false,
            envelope_period: 0,
            envelope_divider: 0,
            envelope_decay_level: 0,
            envelope_start: false,
            mode_short: false,
            shift_register: 1,
            timer_period: NOISE_PERIOD_TABLE[0],
            timer_value: NOISE_PERIOD_TABLE[0],
        }
    }
}

impl Channel for NoiseChannel {
    fn write_register(&mut self, register: usize, value: u8) {
        match register {
            0 => {
                self.length_halt = (value & 0x20) != 0;
                self.constant_volume = (value & 0x10) != 0;
                self.envelope_period = value & 0x0F;
                self.envelope_start = true;
            }
            1 => {}
            2 => {
                self.mode_short = (value & 0x80) != 0;
                let period_index = (value & 0x0F) as usize;
                self.timer_period = NOISE_PERIOD_TABLE[period_index];
                self.timer_value = self.timer_period;
            }
            3 => {
                self.length_counter = LENGTH_TABLE[(value >> 3) as usize];
                self.envelope_start = true;
            }
            _ => {}
        }
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.length_counter = 0;
        }
    }

    fn clock_timer(&mut self) -> Option<u16> {
        if self.timer_value == 0 {
            self.timer_value = self.timer_period;
            let feedback_bit = if self.mode_short { 6 } else { 1 };
            let bit0 = self.shift_register & 1;
            let bitx = (self.shift_register >> feedback_bit) & 1;
            let feedback = bit0 ^ bitx;
            self.shift_register >>= 1;
            self.shift_register |= feedback << 14;
        } else {
            self.timer_value -= 1;
        }
        None
    }

    fn clock_quarter_frame(&mut self) {
        if self.envelope_start {
            self.envelope_start = false;
            self.envelope_decay_level = 15;
            self.envelope_divider = self.envelope_period;
        } else if self.envelope_divider == 0 {
            self.envelope_divider = self.envelope_period;
            if self.envelope_decay_level == 0 {
                if self.length_halt {
                    self.envelope_decay_level = 15;
                }
            } else {
                self.envelope_decay_level -= 1;
            }
        } else {
            self.envelope_divider = self.envelope_divider.saturating_sub(1);
        }
    }

    fn clock_half_frame(&mut self) {
        if self.length_counter > 0 && !self.length_halt {
            self.length_counter -= 1;
        }
    }

    fn output(&self) -> f32 {
        if !self.enabled || self.length_counter == 0 || (self.shift_register & 1) == 1 {
            return 0.0;
        }
        if self.constant_volume {
            (self.envelope_period & 0x0F) as f32
        } else {
            self.envelope_decay_level as f32
        }
    }

    fn active(&self) -> bool {
        self.length_counter > 0
    }
}
