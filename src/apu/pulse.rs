use crate::apu::CPU_CLOCK_NTSC;
use crate::apu::LengthCounter;
use crate::apu::buffer::RingBuffer;
use crate::apu::channel::Channel;
use crate::apu::channel::PlaybackRate;
use crate::apu::channel::Timbre;
use crate::apu::channel::Volume;
use crate::apu::envelope::Envelope;

pub struct PulseChannel {
    pub debug_disable: bool,
    pub output_buffer: RingBuffer,
    pub edge_buffer: RingBuffer,
    pub last_edge: bool,
    pub envelope: Envelope,
    pub length_counter: LengthCounter,

    pub sweep_enabled: bool,
    pub sweep_period: u8,
    pub sweep_divider: u8,
    pub sweep_negate: bool,
    pub sweep_shift: u8,
    pub sweep_reload: bool,
    pub sweep_ones_compliment: bool,

    pub duty: u8,
    pub sequence_counter: u8,
    pub period_initial: u16,
    pub period_current: u16,
}

impl PulseChannel {
    pub fn new(sweep_ones_compliment: bool) -> PulseChannel {
        return PulseChannel {
            debug_disable: false,
            output_buffer: RingBuffer::new(32768),
            edge_buffer: RingBuffer::new(32768),
            last_edge: false,

            envelope: Envelope::new(),
            length_counter: LengthCounter::new(),

            sweep_enabled: false,
            sweep_period: 0,
            sweep_divider: 0,
            sweep_negate: false,
            sweep_shift: 0,
            sweep_reload: false,
            sweep_ones_compliment: sweep_ones_compliment,

            duty: 0b0000_0001,
            sequence_counter: 0,
            period_initial: 0,
            period_current: 0,
        };
    }

    pub fn clock(&mut self) {
        if self.period_current == 0 {
            self.period_current = self.period_initial;

            if self.sequence_counter == 0 {
                self.sequence_counter = 7;
                self.last_edge = true;
            } else {
                self.sequence_counter -= 1;
            }
        } else {
            self.period_current -= 1;
        }
    }

    pub fn output(&self) -> i16 {
        if self.length_counter.length > 0 {
            let target_period = self.target_period();
            if target_period > 0x7FF || self.period_initial < 8 {
                return 0;
            } else {
                let mut sample = ((self.duty >> self.sequence_counter) & 0b1) as i16;
                sample *= self.envelope.current_volume() as i16;
                return sample;
            }
        } else {
            return 0;
        }
    }

    pub fn target_period(&self) -> u16 {
        let change_amount = self.period_initial >> self.sweep_shift;
        if self.sweep_negate {
            if self.sweep_ones_compliment {
                if self.sweep_shift == 0 || self.period_initial == 0 {
                    return 0;
                }
                return self.period_initial - change_amount - 1;
            } else {
                return self.period_initial - change_amount;
            }
        } else {
            return self.period_initial + change_amount;
        }
    }

    pub fn update_sweep(&mut self) {
        let target_period = self.target_period();
        if self.sweep_divider == 0
            && self.sweep_enabled
            && self.sweep_shift != 0
            && target_period <= 0x7FF
            && self.period_initial >= 8
        {
            self.period_initial = target_period;
        }
        if self.sweep_divider == 0 || self.sweep_reload {
            self.sweep_divider = self.sweep_period;
            self.sweep_reload = false;
        } else {
            self.sweep_divider -= 1;
        }
    }
}

impl Channel for PulseChannel {
    fn sample_buffer(&self) -> &RingBuffer {
        return &self.output_buffer;
    }

    fn edge_buffer(&self) -> &RingBuffer {
        return &self.edge_buffer;
    }

    fn record_current_output(&mut self) {
        self.output_buffer
            .push((self.output() as f32 * -4.0) as i16);
        self.edge_buffer.push(self.last_edge as i16);
        self.last_edge = false;
    }

    fn min_sample(&self) -> i16 {
        return -60;
    }

    fn max_sample(&self) -> i16 {
        return 60;
    }

    fn muted(&self) -> bool {
        return self.debug_disable;
    }

    fn mute(&mut self) {
        self.debug_disable = true;
    }

    fn unmute(&mut self) {
        self.debug_disable = false;
    }

    fn playing(&self) -> bool {
        return (self.length_counter.length > 0)
            && (self.target_period() <= 0x7FF)
            && (self.period_initial > 8)
            && (self.envelope.current_volume() > 0);
    }

    fn rate(&self) -> PlaybackRate {
        let frequency = CPU_CLOCK_NTSC as f32 / (16.0 * (self.period_initial as f32 + 1.0));
        return PlaybackRate::SampleRate {
            frequency: frequency,
        };
    }

    fn volume(&self) -> Option<Volume> {
        return Some(Volume::VolumeIndex {
            index: self.envelope.current_volume() as usize,
            max: 15,
        });
    }

    fn timbre(&self) -> Option<Timbre> {
        return match self.duty {
            0b1000_0000 => Some(Timbre::DutyIndex { index: 0, max: 3 }),
            0b1100_0000 => Some(Timbre::DutyIndex { index: 1, max: 3 }),
            0b1111_0000 => Some(Timbre::DutyIndex { index: 2, max: 3 }),
            0b0011_1111 => Some(Timbre::DutyIndex { index: 3, max: 3 }),
            _ => None,
        };
    }
}
