use crate::apu::LengthCounter;
use crate::apu::buffer::RingBuffer;
use crate::apu::channel::{Channel, PlaybackRate, Timbre, Volume};
use crate::apu::envelope::Envelope;

pub const NOISE_PERIOD_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

pub struct NoiseChannel {
    pub debug_disable: bool,
    pub output_buffer: RingBuffer,
    pub edge_buffer: RingBuffer,
    pub last_edge: bool,

    pub envelope: Envelope,
    pub length_counter: LengthCounter,

    pub mode: u8,
    pub period_initial: u16,
    pub period_current: u16,

    pub shift_register: u16,
}

impl NoiseChannel {
    pub fn new() -> Self {
        NoiseChannel {
            debug_disable: false,
            output_buffer: RingBuffer::new(32768),
            edge_buffer: RingBuffer::new(32768),
            last_edge: false,

            envelope: Envelope::new(),
            length_counter: LengthCounter::new(),

            mode: 0,
            period_initial: NOISE_PERIOD_TABLE[0],
            period_current: NOISE_PERIOD_TABLE[0],

            shift_register: 1,
        }
    }

    pub fn clock(&mut self) {
        if self.period_current == 0 {
            self.period_current = self.period_initial.saturating_sub(1);

            let mut feedback = self.shift_register & 0b1;
            if self.mode == 1 {
                feedback ^= (self.shift_register >> 6) & 0b1;
            } else {
                feedback ^= (self.shift_register >> 1) & 0b1;
            }
            self.shift_register >>= 1;
            self.shift_register |= feedback << 14;
            self.last_edge = true;
        } else {
            self.period_current = self.period_current.saturating_sub(1);
        }
    }

    pub fn output(&self) -> i16 {
        if self.length_counter.length > 0 {
            let mut sample = (self.shift_register & 0b1) as i16;
            sample *= self.envelope.current_volume() as i16;
            return sample;
        } else {
            return 0;
        }
    }
}

impl Channel for NoiseChannel {
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
        -60
    }

    fn max_sample(&self) -> i16 {
        60
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
        (self.length_counter.length > 0) && (self.envelope.current_volume() > 0)
    }

    fn rate(&self) -> PlaybackRate {
        PlaybackRate::Unknown
    }

    fn volume(&self) -> Option<Volume> {
        Some(Volume::VolumeIndex {
            index: self.envelope.current_volume() as usize,
            max: 15,
        })
    }

    fn timbre(&self) -> Option<Timbre> {
        Some(Timbre::DutyIndex {
            index: self.mode as usize,
            max: 1,
        })
    }
}
