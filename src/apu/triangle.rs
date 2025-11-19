use crate::apu::buffer::RingBuffer;
use crate::apu::channel::{Channel, PlaybackRate, Timbre, Volume};
use crate::apu::{CPU_CLOCK_NTSC, LengthCounter};

pub struct TriangleChannel {
    pub debug_disable: bool,
    pub output_buffer: RingBuffer,
    pub edge_buffer: RingBuffer,
    pub last_edge: bool,
    pub length_counter: LengthCounter,

    pub control_flag: bool,
    pub linear_reload_flag: bool,
    pub linear_counter_initial: u8,
    pub linear_counter_current: u8,

    pub sequence_counter: u8,
    pub period_initial: u16,
    pub period_current: u16,
}

impl TriangleChannel {
    pub fn new() -> TriangleChannel {
        TriangleChannel {
            debug_disable: false,
            output_buffer: RingBuffer::new(32768),
            edge_buffer: RingBuffer::new(32768),
            last_edge: false,
            length_counter: LengthCounter::new(),
            control_flag: false,
            linear_reload_flag: false,
            linear_counter_initial: 0,
            linear_counter_current: 0,
            sequence_counter: 0,
            period_initial: 0,
            period_current: 0,
        }
    }

    pub fn update_linear_counter(&mut self) {
        if self.linear_reload_flag {
            self.linear_counter_current = self.linear_counter_initial;
        } else if self.linear_counter_current > 0 {
            self.linear_counter_current -= 1;
        }
        if !self.control_flag {
            self.linear_reload_flag = false;
        }
    }

    pub fn clock(&mut self) {
        if self.linear_counter_current != 0 && self.length_counter.length > 0 {
            if self.period_current == 0 {
                self.period_current = self.period_initial;
                if self.sequence_counter >= 31 {
                    self.sequence_counter = 0;
                    self.last_edge = true;
                } else {
                    self.sequence_counter += 1;
                }
            } else {
                self.period_current -= 1;
            }
        }
    }

    pub fn output(&self) -> i16 {
        if self.period_initial <= 2 {
            7
        } else {
            let triangle_sequence = [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 15, 14, 13, 12, 11, 10, 9, 8,
                7, 6, 5, 4, 3, 2, 1, 0,
            ];
            triangle_sequence[self.sequence_counter as usize]
        }
    }
}

impl Channel for TriangleChannel {
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
        self.length_counter.length > 0
            && self.linear_counter_current != 0
            && self.period_initial > 2
    }

    fn rate(&self) -> PlaybackRate {
        let frequency = CPU_CLOCK_NTSC as f32 / (32.0 * (self.period_initial as f32 + 1.0));
        PlaybackRate::SampleRate { frequency }
    }

    fn volume(&self) -> Option<Volume> {
        None
    }

    fn timbre(&self) -> Option<Timbre> {
        None
    }

    fn amplitude(&self) -> f32 {
        if self.playing() { 0.55 } else { 0.0 }
    }
}
