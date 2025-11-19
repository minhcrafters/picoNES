#[derive(Debug, Clone, Copy)]
pub struct Envelope {
    loop_flag: bool,
    constant_volume: bool,
    volume: u8,
    start_flag: bool,
    divider: u8,
    decay_level: u8,
}

impl Envelope {
    pub fn new() -> Self {
        Envelope {
            loop_flag: false,
            constant_volume: false,
            volume: 0,
            start_flag: false,
            divider: 0,
            decay_level: 0,
        }
    }

    /// Update parameters from the $400x control register.
    /// See https://www.nesdev.org/wiki/APU_Envelope for the exact bit layout.
    pub fn write_control(&mut self, value: u8) {
        self.loop_flag = (value & 0x20) != 0;
        self.constant_volume = (value & 0x10) != 0;
        self.volume = value & 0x0F;
    }

    /// Signal a new start event (triggered by writes to $4003/$4007/$400B).
    pub fn restart(&mut self) {
        self.start_flag = true;
    }

    pub fn clock(&mut self) {
        if self.start_flag {
            self.start_flag = false;
            self.decay_level = 15;
            self.divider = self.reload_value();
            return;
        }

        if self.divider == 0 {
            self.divider = self.reload_value();
            if self.decay_level == 0 {
                if self.loop_flag {
                    self.decay_level = 15;
                }
            } else {
                self.decay_level -= 1;
            }
        } else {
            self.divider = self.divider.saturating_sub(1);
        }
    }

    pub fn output(&self) -> u8 {
        if self.constant_volume {
            self.volume
        } else {
            self.decay_level
        }
    }

    fn reload_value(&self) -> u8 {
        self.volume.saturating_add(1)
    }
}
