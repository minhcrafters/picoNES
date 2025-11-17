use crate::apu::channel::Channel;

use super::LENGTH_TABLE;

const PULSE_DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0],
    [0, 1, 1, 0, 0, 0, 0, 0],
    [0, 1, 1, 1, 1, 0, 0, 0],
    [1, 0, 0, 1, 1, 1, 1, 1],
];

#[derive(Debug, Clone)]
pub(super) struct PulseChannel {
    enabled: bool,
    duty: u8,
    duty_position: usize,
    timer_period: u16,
    timer_value: u16,
    length_counter: u8,
    length_halt: bool,
    constant_volume: bool,
    envelope_period: u8,
    envelope_divider: u8,
    envelope_decay_level: u8,
    envelope_start: bool,
    sweep_enabled: bool,
    sweep_period: u8,
    sweep_counter: u8,
    sweep_shift: u8,
    sweep_negate: bool,
    sweep_reload: bool,
    negate_correction: u16,
}

impl PulseChannel {
    pub(super) fn new(negate_correction: u16) -> Self {
        PulseChannel {
            enabled: false,
            duty: 0,
            duty_position: 0,
            timer_period: 0,
            timer_value: 0,
            length_counter: 0,
            length_halt: false,
            constant_volume: false,
            envelope_period: 0,
            envelope_divider: 0,
            envelope_decay_level: 0,
            envelope_start: false,
            sweep_enabled: false,
            sweep_period: 1,
            sweep_counter: 0,
            sweep_shift: 0,
            sweep_negate: false,
            sweep_reload: false,
            negate_correction,
        }
    }

    pub(super) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.length_counter = 0;
        }
    }

    pub(super) fn write_register(&mut self, register: usize, value: u8) {
        match register {
            0 => {
                self.duty = (value >> 6) & 0x03;
                self.length_halt = (value & 0x20) != 0;
                self.constant_volume = (value & 0x10) != 0;
                self.envelope_period = value & 0x0F;
                self.envelope_start = true;
            }
            1 => {
                self.sweep_enabled = (value & 0x80) != 0;
                self.sweep_period = ((value >> 4) & 0x07) + 1;
                self.sweep_negate = (value & 0x08) != 0;
                self.sweep_shift = value & 0x07;
                self.sweep_reload = true;
            }
            2 => {
                self.timer_period = (self.timer_period & 0xFF00) | value as u16;
            }
            3 => {
                self.timer_period = (self.timer_period & 0x00FF) | (((value & 0x07) as u16) << 8);
                self.timer_value = self.timer_period;
                self.length_counter = LENGTH_TABLE[(value >> 3) as usize];
                self.envelope_start = true;
                self.duty_position = 0;
            }
            _ => {}
        }
    }

    pub(super) fn clock_timer(&mut self) {
        if self.timer_period < 8 || self.timer_period > 0x07FF {
            return;
        }

        if self.timer_value == 0 {
            self.timer_value = self.timer_period;
            self.duty_position = (self.duty_position + 1) & 0x07;
        } else {
            self.timer_value -= 1;
        }
    }

    pub(super) fn clock_quarter_frame(&mut self) {
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

    pub(super) fn clock_half_frame(&mut self) {
        if self.length_counter > 0 && !self.length_halt {
            self.length_counter -= 1;
        }

        if self.sweep_counter == 0 {
            self.sweep_counter = self.sweep_period;
            if self.sweep_enabled && self.sweep_shift > 0 && !self.sweep_mute() {
                let target = self.sweep_target_period();
                if target <= 0x07FF {
                    self.timer_period = target;
                }
            }
        } else {
            self.sweep_counter -= 1;
        }

        if self.sweep_reload {
            self.sweep_counter = self.sweep_period;
            self.sweep_reload = false;
        }
    }

    pub(super) fn output(&self) -> f32 {
        if !self.enabled || self.length_counter == 0 || self.sweep_mute() {
            return 0.0;
        }
        if PULSE_DUTY_TABLE[self.duty as usize][self.duty_position] == 0 {
            return 0.0;
        }
        if self.constant_volume {
            (self.envelope_period & 0x0F) as f32
        } else {
            self.envelope_decay_level as f32
        }
    }

    pub(super) fn is_active(&self) -> bool {
        self.length_counter > 0
    }

    fn sweep_target_period(&self) -> u16 {
        let change = self.timer_period >> self.sweep_shift;
        if self.sweep_negate {
            self.timer_period
                .saturating_sub(change + self.negate_correction)
        } else {
            self.timer_period.saturating_add(change)
        }
    }

    fn sweep_mute(&self) -> bool {
        if self.timer_period < 8 || self.timer_period > 0x07FF {
            return true;
        }

        if self.sweep_enabled && self.sweep_shift > 0 {
            let change = self.timer_period >> self.sweep_shift;
            if self.sweep_negate && change + self.negate_correction > self.timer_period {
                return true;
            }

            return self.sweep_target_period() > 0x07FF;
        }

        false
    }
}

impl Channel for PulseChannel {
    fn write_register(&mut self, register: usize, value: u8) {
        self.write_register(register, value);
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.set_enabled(enabled);
    }

    fn clock_timer(&mut self) -> Option<u16> {
        self.clock_timer();
        None
    }

    fn clock_quarter_frame(&mut self) {
        self.clock_quarter_frame();
    }

    fn clock_half_frame(&mut self) {
        self.clock_half_frame();
    }

    fn output(&self) -> f32 {
        self.output()
    }

    fn active(&self) -> bool {
        self.is_active()
    }
}
