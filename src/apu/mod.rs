use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

mod channel;
mod dmc;
mod noise;
mod pulse;
mod triangle;

use channel::Channel;
use dmc::DmcChannel;
use noise::NoiseChannel;
use triangle::TriangleChannel;

use pulse::PulseChannel;

const CPU_FREQUENCY_NTSC: f64 = 1_789_773.0;
const FOUR_STEP_PERIOD: u32 = 14_916;
const FIVE_STEP_PERIOD: u32 = 18_640;

pub(crate) const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

pub struct APU {
    pulse1: PulseChannel,
    pulse2: PulseChannel,
    triangle: TriangleChannel,
    noise: NoiseChannel,
    dmc: DmcChannel,
    frame_counter: FrameCounter,
    sample_interval: f64,
    sample_timer: f64,
    audio_buffer: Arc<Mutex<VecDeque<f32>>>,
    max_buffer_samples: usize,
    cycle_parity: bool,
}

impl APU {
    pub fn new(sample_rate: u32, audio_buffer: Arc<Mutex<VecDeque<f32>>>) -> Self {
        let sr = sample_rate.max(1);
        let interval = CPU_FREQUENCY_NTSC / sr as f64;
        APU {
            pulse1: PulseChannel::new(1),
            pulse2: PulseChannel::new(0),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            dmc: DmcChannel::new(),
            frame_counter: FrameCounter::new(),
            sample_interval: interval,
            sample_timer: 0.0,
            audio_buffer,
            max_buffer_samples: (sr as usize).saturating_mul(4),
            cycle_parity: false,
        }
    }

    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0x4000..=0x4003 => {
                self.pulse1.write_register((addr - 0x4000) as usize, value);
            }
            0x4004..=0x4007 => {
                self.pulse2.write_register((addr - 0x4004) as usize, value);
            }
            0x4008..=0x400B => {
                self.triangle
                    .write_register((addr - 0x4008) as usize, value);
            }
            0x400C..=0x400F => {
                self.noise.write_register((addr - 0x400C) as usize, value);
            }
            0x4010..=0x4013 => {
                self.dmc.write_register((addr - 0x4010) as usize, value);
            }
            _ => {}
        }
    }

    pub fn write_status(&mut self, value: u8) {
        self.pulse1.set_enabled(value & 0x01 != 0);
        self.pulse2.set_enabled(value & 0x02 != 0);
        self.triangle.set_enabled(value & 0x04 != 0);
        self.noise.set_enabled(value & 0x08 != 0);
        self.dmc.set_enabled(value & 0x10 != 0);

        if value & 0x10 == 0 {
            self.dmc.clear_irq();
        }
    }

    pub fn read_status(&mut self) -> u8 {
        let mut status = 0u8;
        if self.pulse1.active() {
            status |= 0x01;
        }
        if self.pulse2.active() {
            status |= 0x02;
        }
        if self.triangle.active() {
            status |= 0x04;
        }
        if self.noise.active() {
            status |= 0x08;
        }
        if self.dmc.active() {
            status |= 0x10;
        }
        if self.frame_counter.irq_flag() {
            status |= 0x40;
            self.frame_counter.clear_irq();
        }
        if self.dmc.irq_flag() {
            status |= 0x80;
            self.dmc.clear_irq();
        }
        status
    }

    pub fn write_frame_counter(&mut self, value: u8) {
        let five_step = (value & 0x80) != 0;
        let irq_inhibit = (value & 0x40) != 0;
        let immediate = self.frame_counter.set_control(five_step, irq_inhibit);

        if immediate.quarter {
            self.clock_quarter_frame();
        }
        if immediate.half {
            self.clock_half_frame();
        }
    }

    pub fn provide_dmc_sample(&mut self, value: u8) {
        self.dmc.provide_sample(value);
    }

    pub fn poll_irq(&mut self) -> Option<u8> {
        if self.frame_counter.irq_flag() || self.dmc.irq_flag() {
            Some(0)
        } else {
            None
        }
    }

    pub fn clock(&mut self) -> Option<u16> {
        let dma_request = self.dmc.clock_timer();

        if self.cycle_parity {
            self.pulse1.clock_timer();
            self.pulse2.clock_timer();
            self.noise.clock_timer();
        }
        self.triangle.clock_timer();
        self.cycle_parity = !self.cycle_parity;

        let frame_event = self.frame_counter.clock();
        if frame_event.quarter {
            self.clock_quarter_frame();
        }
        if frame_event.half {
            self.clock_half_frame();
        }

        self.sample_timer += 1.0;
        while self.sample_timer >= self.sample_interval {
            self.sample_timer -= self.sample_interval;
            let sample = self.mix_output();
            self.push_sample(sample);
        }

        dma_request
    }

    fn clock_quarter_frame(&mut self) {
        self.pulse1.clock_quarter_frame();
        self.pulse2.clock_quarter_frame();
        self.triangle.clock_quarter_frame();
        self.noise.clock_quarter_frame();
    }

    fn clock_half_frame(&mut self) {
        self.pulse1.clock_half_frame();
        self.pulse2.clock_half_frame();
        self.triangle.clock_half_frame();
        self.noise.clock_half_frame();
    }

    fn mix_output(&self) -> f32 {
        let pulse_total = self.pulse1.output() + self.pulse2.output();
        let pulse_out = if pulse_total == 0.0 {
            0.0
        } else {
            95.88 / (8128.0 / pulse_total + 100.0)
        };

        let t = self.triangle.output();
        let n = self.noise.output();
        let d = self.dmc.output();
        let tnd_input = (t / 8227.0) + (n / 12241.0) + (d / 22638.0);
        let tnd_out = if tnd_input == 0.0 {
            0.0
        } else {
            159.79 / (1.0 / tnd_input + 100.0)
        };

        ((pulse_out + tnd_out) * 2.0 - 1.0) as f32
    }

    fn push_sample(&mut self, sample: f32) {
        if let Ok(mut buffer) = self.audio_buffer.lock() {
            if buffer.len() >= self.max_buffer_samples {
                let _ = buffer.pop_front();
            }
            buffer.push_back(sample);
        }
    }
}

#[derive(Default)]
struct FrameEvent {
    quarter: bool,
    half: bool,
}

#[derive(Clone, Copy)]
struct FrameStep {
    cycle: u32,
    quarter: bool,
    half: bool,
    irq: bool,
}

const FOUR_STEP_SEQUENCE: [FrameStep; 4] = [
    FrameStep {
        cycle: 3729,
        quarter: true,
        half: false,
        irq: false,
    },
    FrameStep {
        cycle: 7457,
        quarter: true,
        half: true,
        irq: false,
    },
    FrameStep {
        cycle: 11_186,
        quarter: true,
        half: false,
        irq: false,
    },
    FrameStep {
        cycle: 14_916,
        quarter: true,
        half: true,
        irq: true,
    },
];

const FIVE_STEP_SEQUENCE: [FrameStep; 5] = [
    FrameStep {
        cycle: 3729,
        quarter: true,
        half: false,
        irq: false,
    },
    FrameStep {
        cycle: 7457,
        quarter: true,
        half: true,
        irq: false,
    },
    FrameStep {
        cycle: 11_186,
        quarter: true,
        half: false,
        irq: false,
    },
    FrameStep {
        cycle: 14_916,
        quarter: true,
        half: true,
        irq: false,
    },
    FrameStep {
        cycle: 18_640,
        quarter: true,
        half: false,
        irq: false,
    },
];

enum FrameCounterMode {
    FourStep,
    FiveStep,
}

struct FrameCounter {
    mode: FrameCounterMode,
    cycle: u32,
    irq_inhibit: bool,
    irq_flag: bool,
}

impl FrameCounter {
    fn new() -> Self {
        FrameCounter {
            mode: FrameCounterMode::FourStep,
            cycle: 0,
            irq_inhibit: false,
            irq_flag: false,
        }
    }

    fn clock(&mut self) -> FrameEvent {
        self.cycle = self.cycle.wrapping_add(1);
        let mut event = FrameEvent::default();

        let (sequence, period) = match self.mode {
            FrameCounterMode::FourStep => (&FOUR_STEP_SEQUENCE[..], FOUR_STEP_PERIOD),
            FrameCounterMode::FiveStep => (&FIVE_STEP_SEQUENCE[..], FIVE_STEP_PERIOD),
        };

        for step in sequence {
            if self.cycle == step.cycle {
                if step.quarter {
                    event.quarter = true;
                }
                if step.half {
                    event.half = true;
                }
                if step.irq && !self.irq_inhibit {
                    self.irq_flag = true;
                }
            }
        }

        if self.cycle >= period {
            self.cycle = 0;
        }

        event
    }

    fn set_control(&mut self, five_step: bool, irq_inhibit: bool) -> FrameEvent {
        self.mode = if five_step {
            FrameCounterMode::FiveStep
        } else {
            FrameCounterMode::FourStep
        };
        self.irq_inhibit = irq_inhibit;
        self.irq_flag = false;
        self.cycle = 0;

        if five_step {
            FrameEvent {
                quarter: true,
                half: true,
            }
        } else {
            FrameEvent::default()
        }
    }

    fn irq_flag(&self) -> bool {
        self.irq_flag
    }

    fn clear_irq(&mut self) {
        self.irq_flag = false;
    }
}
