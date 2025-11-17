use super::channel::Channel;

const DMC_RATE_TABLE: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 85, 72, 54,
];

#[derive(Debug)]
pub struct DmcChannel {
    enabled: bool,
    irq_enabled: bool,
    loop_flag: bool,
    rate_index: u8,
    timer_period: u16,
    timer_value: u16,
    output_level: u8,
    sample_address: u16,
    sample_length: u16,
    current_address: u16,
    bytes_remaining: u16,
    sample_buffer: Option<u8>,
    shift_register: u8,
    bits_remaining: u8,
    silence: bool,
    irq_flag: bool,
    sample_fetch_pending: bool,
}

impl DmcChannel {
    pub fn new() -> Self {
        Self {
            enabled: false,
            irq_enabled: false,
            loop_flag: false,
            rate_index: 0,
            timer_period: DMC_RATE_TABLE[0],
            timer_value: DMC_RATE_TABLE[0],
            output_level: 0,
            sample_address: 0xC000,
            sample_length: 1,
            current_address: 0xC000,
            bytes_remaining: 0,
            sample_buffer: None,
            shift_register: 0,
            bits_remaining: 8,
            silence: true,
            irq_flag: false,
            sample_fetch_pending: false,
        }
    }

    fn reload_sample(&mut self) {
        self.current_address = self.sample_address;
        self.bytes_remaining = self.sample_length;
    }

    fn timer_reload(&mut self) {
        self.timer_value = self.timer_period;
    }

    fn step_output(&mut self) {
        if self.silence {
            return;
        }
        if self.shift_register & 0x01 != 0 {
            if self.output_level <= 125 {
                self.output_level += 2;
            }
        } else if self.output_level >= 2 {
            self.output_level -= 2;
        }
    }

    fn fetch_next_sample(&mut self) -> Option<u16> {
        if self.sample_fetch_pending {
            return None;
        }
        if self.sample_buffer.is_none() && self.bytes_remaining > 0 {
            self.sample_fetch_pending = true;
            Some(self.current_address)
        } else {
            None
        }
    }

    fn advance_reader(&mut self) {
        self.current_address = self.current_address.wrapping_add(1);
        if self.current_address == 0 {
            self.current_address = 0x8000;
        }

        if self.bytes_remaining > 0 {
            self.bytes_remaining -= 1;
        }

        if self.bytes_remaining == 0 {
            if self.loop_flag {
                self.reload_sample();
            } else {
                if self.irq_enabled {
                    self.irq_flag = true;
                }
            }
        }
    }
}

impl Channel for DmcChannel {
    fn write_register(&mut self, register: usize, value: u8) {
        match register {
            0 => {
                self.irq_enabled = (value & 0x80) != 0;
                if !self.irq_enabled {
                    self.irq_flag = false;
                }
                self.loop_flag = (value & 0x40) != 0;
                self.rate_index = value & 0x0F;
                self.timer_period = DMC_RATE_TABLE[self.rate_index as usize];
            }
            1 => {
                self.output_level = value & 0x7F;
            }
            2 => {
                self.sample_address = 0xC000 + ((value as u16) << 6);
                self.current_address = self.sample_address;
            }
            3 => {
                self.sample_length = ((value as u16) << 4) + 1;
            }
            _ => {}
        }
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled {
            if self.bytes_remaining == 0 {
                self.reload_sample();
            }
            if self.bits_remaining == 0 {
                self.bits_remaining = 8;
            }
        } else {
            self.bytes_remaining = 0;
            self.sample_buffer = None;
            self.bits_remaining = 0;
            self.silence = true;
            self.sample_fetch_pending = false;
            self.irq_flag = false;
        }
    }

    fn clock_timer(&mut self) -> Option<u16> {
        if !self.enabled {
            return None;
        }

        if self.timer_value == 0 {
            self.timer_reload();

            if self.bits_remaining == 0 {
                if let Some(buffered) = self.sample_buffer.take() {
                    self.shift_register = buffered;
                    self.bits_remaining = 8;
                    self.silence = false;
                } else {
                    self.silence = true;
                }
            }

            self.step_output();

            if self.bits_remaining > 0 {
                self.shift_register >>= 1;
                self.bits_remaining -= 1;
            }
        } else {
            self.timer_value -= 1;
        }

        self.fetch_next_sample()
    }

    fn clock_quarter_frame(&mut self) {}

    fn clock_half_frame(&mut self) {}

    fn output(&self) -> f32 {
        self.output_level as f32
    }

    fn active(&self) -> bool {
        self.bytes_remaining > 0 || self.sample_buffer.is_some()
    }

    fn irq_flag(&self) -> bool {
        self.irq_flag
    }

    fn clear_irq(&mut self) {
        self.irq_flag = false;
    }

    fn provide_sample(&mut self, value: u8) {
        self.sample_buffer = Some(value);
        self.sample_fetch_pending = false;
        self.advance_reader();
    }
}
