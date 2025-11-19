use crate::apu::buffer::RingBuffer;
use crate::apu::channel::{Channel, PlaybackRate, Timbre, Volume};

pub const DMC_RATE_TABLE: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

pub struct DmcChannel {
    pub debug_disable: bool,
    pub output_buffer: RingBuffer,
    pub edge_buffer: RingBuffer,
    pub last_edge: bool,

    pub looping: bool,
    pub period_initial: u16,
    pub period_current: u16,
    pub output_level: u8,
    pub starting_address: u16,
    pub sample_length: u16,

    pub current_address: u16,
    pub sample_buffer: Option<u8>,
    pub shift_register: u8,
    pub bits_remaining: u8,
    pub bytes_remaining: u16,
    pub silence_flag: bool,

    pub interrupt_enabled: bool,
    pub interrupt_flag: bool,

    pub sample_fetch_pending: bool,
}

impl DmcChannel {
    pub fn new() -> DmcChannel {
        DmcChannel {
            debug_disable: false,
            output_buffer: RingBuffer::new(32768),
            edge_buffer: RingBuffer::new(32768),
            last_edge: false,

            looping: false,
            period_initial: DMC_RATE_TABLE[0],
            period_current: DMC_RATE_TABLE[0],
            output_level: 0,
            starting_address: 0,
            sample_length: 0,

            current_address: 0,
            sample_buffer: None,
            shift_register: 0,
            bits_remaining: 8,
            bytes_remaining: 0,
            silence_flag: false,

            interrupt_enabled: false,
            interrupt_flag: false,

            sample_fetch_pending: false,
        }
    }

    pub fn begin_output_cycle(&mut self) {
        self.bits_remaining = 8;
        if self.sample_buffer.is_none() {
            self.silence_flag = true;
        } else {
            self.silence_flag = false;
            self.shift_register = self.sample_buffer.unwrap_or(0);
            self.sample_buffer = None;
        }
    }

    pub fn update_output_unit(&mut self) {
        if !self.silence_flag {
            let mut target_output = self.output_level;
            if (self.shift_register & 0b1) == 0 {
                if self.output_level >= 2 {
                    target_output = target_output.wrapping_sub(2);
                }
            } else {
                if self.output_level <= 125 {
                    target_output = target_output.wrapping_add(2);
                }
            }
            self.output_level = target_output;
        }
        self.shift_register >>= 1;
        if self.bits_remaining > 0 {
            self.bits_remaining -= 1;
        }
        if self.bits_remaining == 0 {
            self.begin_output_cycle();
        }
    }

    pub fn clock(&mut self) -> Option<u16> {
        if self.period_current == 0 {
            self.period_current = self.period_initial;
            self.update_output_unit();
        } else {
            self.period_current = self.period_current.saturating_sub(1);
        }

        if !self.sample_fetch_pending && self.sample_buffer.is_none() && self.bytes_remaining > 0 {
            self.sample_fetch_pending = true;
            Some(0x8000u16 | (self.current_address & 0x7FFF))
        } else {
            None
        }
    }

    pub fn provide_sample(&mut self, value: u8) {
        self.sample_buffer = Some(value);
        self.sample_fetch_pending = false;
        self.current_address = self.current_address.wrapping_add(1);
        if self.bytes_remaining > 0 {
            self.bytes_remaining = self.bytes_remaining.saturating_sub(1);
        }
        if self.bytes_remaining == 0 {
            if self.looping {
                self.current_address = self.starting_address;
                self.bytes_remaining = self.sample_length;
                self.last_edge = true;
            } else if self.interrupt_enabled {
                self.interrupt_flag = true;
            }
        }
    }

    pub fn output(&self) -> i16 {
        self.output_level as i16
    }
}

impl Channel for DmcChannel {
    fn sample_buffer(&self) -> &RingBuffer {
        &self.output_buffer
    }

    fn edge_buffer(&self) -> &RingBuffer {
        &self.edge_buffer
    }

    fn record_current_output(&mut self) {
        self.output_buffer
            .push((self.output() as f32 * -4.0) as i16);
        self.edge_buffer.push(self.last_edge as i16);
        self.last_edge = false;
    }

    fn min_sample(&self) -> i16 {
        -512
    }

    fn max_sample(&self) -> i16 {
        512
    }

    fn muted(&self) -> bool {
        self.debug_disable
    }

    fn mute(&mut self) {
        self.debug_disable = true;
    }

    fn unmute(&mut self) {
        self.debug_disable = false;
    }

    fn playing(&self) -> bool {
        self.bytes_remaining > 0 || self.output_level > 0
    }

    fn rate(&self) -> PlaybackRate {
        PlaybackRate::Unknown
    }

    fn volume(&self) -> Option<Volume> {
        None
    }

    fn timbre(&self) -> Option<Timbre> {
        None
    }
}
