use super::{channel::Channel, envelope::Envelope, LENGTH_TABLE};

const NOISE_PERIOD_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

#[derive(Debug)]
pub struct NoiseChannel {
    enabled: bool,
    length_counter: u8,
    length_halt: bool,
    envelope: Envelope,
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
            envelope: Envelope::new(),
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
                self.envelope.write_control(value);
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
                self.envelope.restart();
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
        self.envelope.clock();
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
        self.envelope.output() as f32
    }

    fn active(&self) -> bool {
        self.length_counter > 0
    }
}
