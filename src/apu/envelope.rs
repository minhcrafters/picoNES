#[derive(Clone, Copy)]
pub struct Envelope {
    pub looping: bool,
    pub enabled: bool,
    pub start_flag: bool,
    pub divider: u8,
    pub decay_level_counter: u8,
    pub volume_register: u8,
}

impl Envelope {
    pub fn new() -> Self {
        Envelope {
            looping: false,
            enabled: true,
            start_flag: false,
            divider: 0,
            decay_level_counter: 0,
            volume_register: 0,
        }
    }

    pub fn clock(&mut self) {
        if self.start_flag {
            self.start_flag = false;
            self.decay_level_counter = 15;
            self.divider = self.reload_value();
            return;
        }

        if self.divider == 0 {
            self.divider = self.reload_value();
            if self.decay_level_counter == 0 {
                if self.looping {
                    self.decay_level_counter = 15;
                }
            } else {
                self.decay_level_counter -= 1;
            }
        } else {
            self.divider -= 1;
        }
    }

    pub fn current_volume(&self) -> u8 {
        if self.enabled {
            self.decay_level_counter
        } else {
            self.volume_register
        }
    }

    fn reload_value(&self) -> u8 {
        self.volume_register.saturating_add(1)
    }
}
