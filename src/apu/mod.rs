// thanks zeta for original APU implementation

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

mod buffer;
mod channel;
mod dmc;
mod envelope;
mod noise;
mod pulse;
mod triangle;

use channel::Channel;
use dmc::DmcChannel;
use noise::NoiseChannel;
use pulse::PulseChannel;
use triangle::TriangleChannel;

use crate::apu::dmc::DMC_RATE_TABLE;
use crate::apu::noise::NOISE_PERIOD_TABLE;

const CPU_CLOCK_NTSC: u64 = 1_789_773;

pub const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

#[derive(Clone, Copy)]
pub struct LengthCounter {
    pub length: u8,
    pub halt_flag: bool,
    pub channel_enabled: bool,
}

impl LengthCounter {
    pub fn new() -> Self {
        LengthCounter {
            length: 0,
            halt_flag: false,
            channel_enabled: false,
        }
    }

    pub fn clock(&mut self) {
        if self.length > 0 && !self.halt_flag {
            self.length -= 1;
        }
    }

    pub fn set_length(&mut self, index: u8) {
        if self.channel_enabled {
            let idx = index.min((LENGTH_TABLE.len() - 1) as u8) as usize;
            self.length = LENGTH_TABLE[idx];
        }
    }
}

pub struct APU {
    current_cycle: u64,

    frame_sequencer_mode: u8,
    frame_sequencer: u16,
    frame_reset_delay: u8,
    quarter_frame_counter: u32,
    half_frame_counter: u32,

    frame_interrupt: bool,
    disable_interrupt: bool,

    pulse1: PulseChannel,
    pulse2: PulseChannel,
    triangle: TriangleChannel,
    noise: NoiseChannel,
    dmc: DmcChannel,

    sample_rate: u64,
    cpu_clock_rate: u64,
    generated_samples: u64,
    next_sample_at: u64,

    pulse_table: Vec<f32>,
    tnd_table: Vec<f32>,

    audio_buffer: Arc<Mutex<VecDeque<f32>>>,
    max_buffer_samples: usize,

    // DC offset removal filter for click/pop prevention
    dc_filter_x1: f32,
    dc_filter_y1: f32,
}

impl APU {
    pub fn new(sample_rate: u32, audio_buffer: Arc<Mutex<VecDeque<f32>>>) -> Self {
        let sample_rate = sample_rate.max(1) as u64;
        let max_samples = sample_rate as usize * 4;

        APU {
            current_cycle: 0,
            frame_sequencer_mode: 0,
            frame_sequencer: 0,
            frame_reset_delay: 0,
            quarter_frame_counter: 0,
            half_frame_counter: 0,
            frame_interrupt: false,
            disable_interrupt: false,
            pulse1: PulseChannel::new(true),
            pulse2: PulseChannel::new(false),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            dmc: DmcChannel::new(),
            sample_rate,
            cpu_clock_rate: CPU_CLOCK_NTSC,
            generated_samples: 0,
            next_sample_at: 0,
            pulse_table: generate_pulse_table(),
            tnd_table: generate_tnd_table(),
            audio_buffer,
            max_buffer_samples: max_samples,
            dc_filter_x1: 0.0,
            dc_filter_y1: 0.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate.max(1) as u64;
        self.max_buffer_samples = (self.sample_rate as usize).saturating_mul(4);
    }

    pub fn write_register(&mut self, addr: u16, value: u8) {
        let duty_table = [0b1000_0000, 0b1100_0000, 0b1111_0000, 0b0011_1111];
        match addr {
            0x4000 => {
                let duty_index = (value & 0b1100_0000) >> 6;
                let length_disable = (value & 0b0010_0000) != 0;
                let constant_volume = (value & 0b0001_0000) != 0;

                self.pulse1.duty = duty_table[duty_index as usize];
                self.pulse1.length_counter.halt_flag = length_disable;
                self.pulse1.envelope.looping = length_disable;
                self.pulse1.envelope.enabled = !constant_volume;
                self.pulse1.envelope.volume_register = value & 0b0000_1111;
            }
            0x4001 => {
                self.pulse1.sweep_enabled = (value & 0b1000_0000) != 0;
                self.pulse1.sweep_period = (value & 0b0111_0000) >> 4;
                self.pulse1.sweep_negate = (value & 0b0000_1000) != 0;
                self.pulse1.sweep_shift = value & 0b0000_0111;
                self.pulse1.sweep_reload = true;
            }
            0x4002 => {
                let period_low = value as u16;
                self.pulse1.period_initial = (self.pulse1.period_initial & 0xFF00) | period_low;
                self.pulse1.period_current = self.pulse1.period_initial;
            }
            0x4003 => {
                let period_high = ((value & 0b0000_0111) as u16) << 8;
                let length_index = (value & 0b1111_1000) >> 3;

                self.pulse1.period_initial = (self.pulse1.period_initial & 0x00FF) | period_high;
                self.pulse1.period_current = self.pulse1.period_initial;
                self.pulse1.length_counter.set_length(length_index);
                self.pulse1.sequence_counter = 0;
                self.pulse1.envelope.start_flag = true;
            }
            0x4004 => {
                let duty_index = (value & 0b1100_0000) >> 6;
                let length_disable = (value & 0b0010_0000) != 0;
                let constant_volume = (value & 0b0001_0000) != 0;

                self.pulse2.duty = duty_table[duty_index as usize];
                self.pulse2.length_counter.halt_flag = length_disable;
                self.pulse2.envelope.looping = length_disable;
                self.pulse2.envelope.enabled = !constant_volume;
                self.pulse2.envelope.volume_register = value & 0b0000_1111;
            }
            0x4005 => {
                self.pulse2.sweep_enabled = (value & 0b1000_0000) != 0;
                self.pulse2.sweep_period = (value & 0b0111_0000) >> 4;
                self.pulse2.sweep_negate = (value & 0b0000_1000) != 0;
                self.pulse2.sweep_shift = value & 0b0000_0111;
                self.pulse2.sweep_reload = true;
            }
            0x4006 => {
                let period_low = value as u16;
                self.pulse2.period_initial = (self.pulse2.period_initial & 0xFF00) | period_low;
                self.pulse2.period_current = self.pulse2.period_initial;
            }
            0x4007 => {
                let period_high = ((value & 0b0000_0111) as u16) << 8;
                let length_index = (value & 0b1111_1000) >> 3;

                self.pulse2.period_initial = (self.pulse2.period_initial & 0x00FF) | period_high;
                self.pulse2.period_current = self.pulse2.period_initial;
                self.pulse2.length_counter.set_length(length_index);
                self.pulse2.sequence_counter = 0;
                self.pulse2.envelope.start_flag = true;
            }
            0x4008 => {
                self.triangle.control_flag = (value & 0b1000_0000) != 0;
                self.triangle.length_counter.halt_flag = self.triangle.control_flag;
                self.triangle.linear_counter_initial = value & 0b0111_1111;
            }
            0x400A => {
                let period_low = value as u16;
                self.triangle.period_initial = (self.triangle.period_initial & 0xFF00) | period_low;
                self.triangle.period_current = self.triangle.period_initial;
            }
            0x400B => {
                let period_high = ((value & 0b0000_0111) as u16) << 8;
                let length_index = (value & 0b1111_1000) >> 3;

                self.triangle.period_initial =
                    (self.triangle.period_initial & 0x00FF) | period_high;
                self.triangle.period_current = self.triangle.period_initial;
                self.triangle.length_counter.set_length(length_index);
                self.triangle.linear_reload_flag = true;
            }
            0x400C => {
                let length_disable = (value & 0b0010_0000) != 0;
                let constant_volume = (value & 0b0001_0000) != 0;

                self.noise.length_counter.halt_flag = length_disable;
                self.noise.envelope.looping = length_disable;
                self.noise.envelope.enabled = !constant_volume;
                self.noise.envelope.volume_register = value & 0b0000_1111;
            }
            0x400E => {
                let period_index = value & 0b0000_1111;
                self.noise.mode = (value & 0b1000_0000) >> 7;
                self.noise.period_initial = NOISE_PERIOD_TABLE[period_index as usize];
                self.noise.period_current = self.noise.period_initial;
            }
            0x400F => {
                let length_index = (value & 0b1111_1000) >> 3;
                self.noise.length_counter.set_length(length_index);
                self.noise.envelope.start_flag = true;
            }
            0x4010 => {
                self.dmc.looping = (value & 0b0100_0000) != 0;
                self.dmc.interrupt_enabled = (value & 0b1000_0000) != 0;
                if !self.dmc.interrupt_enabled {
                    self.dmc.interrupt_flag = false;
                }
                let period_index = value & 0b0000_1111;
                self.dmc.period_initial = DMC_RATE_TABLE[period_index as usize];
                self.dmc.period_current = self.dmc.period_initial;
            }
            0x4011 => {
                self.dmc.output_level = value & 0b0111_1111;
            }
            0x4012 => {
                self.dmc.starting_address = 0xC000 + ((value as u16) << 6);
                self.dmc.current_address = self.dmc.starting_address;
            }
            0x4013 => {
                self.dmc.sample_length = ((value as u16) << 4) + 1;
            }
            _ => {}
        }
    }

    pub fn write_status(&mut self, value: u8) {
        self.pulse1.length_counter.channel_enabled = (value & 0b0001) != 0;
        self.pulse2.length_counter.channel_enabled = (value & 0b0010) != 0;
        self.triangle.length_counter.channel_enabled = (value & 0b0100) != 0;
        self.noise.length_counter.channel_enabled = (value & 0b1000) != 0;

        if !self.pulse1.length_counter.channel_enabled {
            self.pulse1.length_counter.length = 0;
        }
        if !self.pulse2.length_counter.channel_enabled {
            self.pulse2.length_counter.length = 0;
        }
        if !self.triangle.length_counter.channel_enabled {
            self.triangle.length_counter.length = 0;
        }
        if !self.noise.length_counter.channel_enabled {
            self.noise.length_counter.length = 0;
        }

        let dmc_enable = (value & 0b1_0000) != 0;
        if !dmc_enable {
            self.dmc.bytes_remaining = 0;
        }
        if dmc_enable && self.dmc.bytes_remaining == 0 {
            self.dmc.current_address = self.dmc.starting_address;
            self.dmc.bytes_remaining = self.dmc.sample_length;
        }
        self.dmc.interrupt_flag = false;
    }

    pub fn read_status(&mut self) -> u8 {
        let mut status = 0u8;
        if self.pulse1.length_counter.length > 0 {
            status |= 0x01;
        }
        if self.pulse2.length_counter.length > 0 {
            status |= 0x02;
        }
        if self.triangle.length_counter.length > 0 {
            status |= 0x04;
        }
        if self.noise.length_counter.length > 0 {
            status |= 0x08;
        }
        if self.dmc.bytes_remaining > 0 {
            status |= 0x10;
        }
        if self.frame_interrupt {
            status |= 0x40;
        }
        if self.dmc.interrupt_flag {
            status |= 0x80;
        }
        self.frame_interrupt = false;
        self.dmc.interrupt_flag = false;
        status
    }

    pub fn write_frame_counter(&mut self, value: u8) {
        self.frame_sequencer_mode = (value & 0b1000_0000) >> 7;
        self.disable_interrupt = (value & 0b0100_0000) != 0;
        if (self.current_cycle & 0b1) != 0 {
            self.frame_reset_delay = 3;
        } else {
            self.frame_reset_delay = 4;
        }
        if self.disable_interrupt {
            self.frame_interrupt = false;
        }
    }

    pub fn provide_dmc_sample(&mut self, value: u8) {
        self.dmc.provide_sample(value);
    }

    pub fn poll_irq(&mut self) -> Option<u8> {
        if self.frame_interrupt || self.dmc.interrupt_flag {
            Some(0)
        } else {
            None
        }
    }

    pub fn clock(&mut self) -> Option<u16> {
        self.clock_frame_sequencer();

        self.triangle.clock();

        let dma_request = self.dmc.clock();

        if (self.current_cycle & 0b1) == 0 {
            self.pulse1.clock();
            self.pulse2.clock();
            self.noise.clock();
        }

        let current_sample = self.mix_sample();

        if self.current_cycle >= self.next_sample_at {
            // Ensure sample is within valid range to prevent extreme spikes
            let composite_sample = current_sample.clamp(-1.0, 1.0);
            self.push_sample(composite_sample);

            self.pulse1.record_current_output();
            self.pulse2.record_current_output();
            self.triangle.record_current_output();
            self.noise.record_current_output();
            self.dmc.record_current_output();

            self.generated_samples += 1;
            self.next_sample_at =
                ((self.generated_samples + 1) * self.cpu_clock_rate) / self.sample_rate;
        }

        self.current_cycle += 1;
        dma_request
    }

    fn push_sample(&mut self, sample: f32) {
        if let Ok(mut buffer) = self.audio_buffer.lock() {
            if buffer.len() >= self.max_buffer_samples {
                let _ = buffer.pop_front();
            }
            buffer.push_back(sample);
        }
    }

    fn mix_sample(&mut self) -> f32 {
        let mut combined_pulse = 0;

        if !self.pulse1.debug_disable {
            combined_pulse += self.pulse1.output();
        }
        if !self.pulse2.debug_disable {
            combined_pulse += self.pulse2.output();
        }

        let pulse_output = self.pulse_table[combined_pulse.min(30) as usize];

        let triangle_output = if self.triangle.debug_disable {
            0
        } else {
            self.triangle.output()
        };
        let noise_output = if self.noise.debug_disable {
            0
        } else {
            self.noise.output()
        };
        let dmc_output = if self.dmc.debug_disable {
            0
        } else {
            self.dmc.output()
        };

        let tnd_index = full_tnd_index(
            (triangle_output as usize).min(15),
            (noise_output as usize).min(15),
            (dmc_output as usize).min(127),
        );
        let tnd_output = self.tnd_table[tnd_index];

        let mixed = (pulse_output - 0.5) + (tnd_output - 0.5);

        // Apply DC offset removal filter to eliminate pops and clicks
        // High-pass filter: y = 0.9999 * (y + x - x_prev)
        let dc_alpha = 0.9999;
        let filtered = dc_alpha * (self.dc_filter_y1 + mixed - self.dc_filter_x1);
        self.dc_filter_x1 = mixed;
        self.dc_filter_y1 = filtered;

        filtered
    }

    fn clock_frame_sequencer(&mut self) {
        if self.frame_reset_delay > 0 {
            self.frame_reset_delay -= 1;
            if self.frame_reset_delay == 0 {
                self.frame_sequencer = 0;
                if self.frame_sequencer_mode == 1 {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
            }
        }

        if self.frame_sequencer_mode == 0 {
            match self.frame_sequencer {
                7457 => self.clock_quarter_frame(),
                14913 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
                22371 => self.clock_quarter_frame(),
                29828 => {
                    if !self.disable_interrupt {
                        self.frame_interrupt = true;
                    }
                }
                29829 => {
                    if !self.disable_interrupt {
                        self.frame_interrupt = true;
                    }
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
                29830 => {
                    if !self.disable_interrupt {
                        self.frame_interrupt = true;
                    }
                    self.frame_sequencer = 0;
                }
                _ => {}
            }
        } else {
            match self.frame_sequencer {
                7457 => self.clock_quarter_frame(),
                14913 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
                22371 => self.clock_quarter_frame(),
                37281 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
                37282 => {
                    self.frame_sequencer = 0;
                }
                _ => {}
            }
        }

        self.frame_sequencer += 1;
    }

    fn clock_quarter_frame(&mut self) {
        self.pulse1.envelope.clock();
        self.pulse2.envelope.clock();
        self.triangle.update_linear_counter();
        self.noise.envelope.clock();
        self.quarter_frame_counter = self.quarter_frame_counter.wrapping_add(1);
    }

    fn clock_half_frame(&mut self) {
        self.pulse1.update_sweep();
        self.pulse2.update_sweep();

        self.pulse1.length_counter.clock();
        self.pulse2.length_counter.clock();
        self.triangle.length_counter.clock();
        self.noise.length_counter.clock();
        self.half_frame_counter = self.half_frame_counter.wrapping_add(1);
    }
}

fn generate_pulse_table() -> Vec<f32> {
    let mut pulse_table = vec![0f32; 31];
    for n in 1..31 {
        pulse_table[n] = 95.52 / (8128.0 / (n as f32) + 100.0);
    }
    pulse_table
}

fn full_tnd_index(t: usize, n: usize, d: usize) -> usize {
    3 * t + 2 * n + d
}

fn generate_tnd_table() -> Vec<f32> {
    let mut tnd_table = vec![0f32; 203];
    for n in 1..203 {
        tnd_table[n] = 163.67 / (24329.0 / n as f32 + 100.0);
    }
    tnd_table
}
