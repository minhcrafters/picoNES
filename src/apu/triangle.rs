use super::{LENGTH_TABLE, channel::Channel};

const TRIANGLE_SEQUENCE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    13, 14, 15,
];

#[derive(Debug)]
pub struct TriangleChannel {
    enabled: bool,
    control_flag: bool,
    linear_reload_value: u8,
    linear_counter: u8,
    linear_reload_flag: bool,
    timer_period: u16,
    timer_value: u16,
    length_counter: u8,
    sequence_index: usize,
}

impl TriangleChannel {
    pub fn new() -> Self {
        Self {
            enabled: false,
            control_flag: false,
            linear_reload_value: 0,
            linear_counter: 0,
            linear_reload_flag: false,
            timer_period: 0,
            timer_value: 0,
            length_counter: 0,
            sequence_index: 0,
        }
    }
}

impl Channel for TriangleChannel {
    fn write_register(&mut self, register: usize, value: u8) {
        match register {
            0 => {
                self.control_flag = (value & 0x80) != 0;
                self.linear_reload_value = value & 0x7F;
                self.linear_reload_flag = true;
            }
            1 => {}
            2 => {
                self.timer_period = (self.timer_period & 0xFF00) | value as u16;
            }
            3 => {
                self.timer_period = (self.timer_period & 0x00FF) | (((value & 0x07) as u16) << 8);
                self.timer_value = self.timer_period;
                self.length_counter = LENGTH_TABLE[(value >> 3) as usize];
                self.linear_reload_flag = true;
                self.sequence_index = 0;
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
        if self.timer_period < 2 || self.timer_period > 0x07FF {
            return None;
        }

        if self.timer_value == 0 {
            self.timer_value = self.timer_period;
            if self.length_counter > 0 && self.linear_counter > 0 {
                self.sequence_index = (self.sequence_index + 1) % TRIANGLE_SEQUENCE.len();
            }
        } else {
            self.timer_value -= 1;
        }

        None
    }

    fn clock_quarter_frame(&mut self) {
        if self.linear_reload_flag {
            self.linear_counter = self.linear_reload_value;
        } else if self.linear_counter > 0 {
            self.linear_counter -= 1;
        }

        if !self.control_flag {
            self.linear_reload_flag = false;
        }
    }

    fn clock_half_frame(&mut self) {
        if self.length_counter > 0 && !self.control_flag {
            self.length_counter -= 1;
        }
    }

    fn output(&self) -> f32 {
        if !self.enabled || self.length_counter == 0 || self.linear_counter == 0 {
            return 0.0;
        }
        TRIANGLE_SEQUENCE[self.sequence_index] as f32
    }

    fn active(&self) -> bool {
        self.length_counter > 0
    }
}
