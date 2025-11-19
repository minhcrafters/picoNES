use crate::apu::buffer::RingBuffer;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum PlaybackRate {
    Unknown,
    SampleRate { frequency: f32 },
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum Volume {
    VolumeIndex { index: usize, max: usize },
    Linear { level: f32 },
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum Timbre {
    DutyIndex { index: usize, max: usize },
}

/// Shared debug/inspection interface exposed by every APU channel.
#[allow(dead_code)]
pub trait Channel {
    fn sample_buffer(&self) -> &RingBuffer;
    fn edge_buffer(&self) -> &RingBuffer;

    fn record_current_output(&mut self);

    fn min_sample(&self) -> i16;
    fn max_sample(&self) -> i16;

    fn muted(&self) -> bool;
    fn mute(&mut self);
    fn unmute(&mut self);

    fn playing(&self) -> bool {
        return false;
    }
    fn rate(&self) -> PlaybackRate {
        return PlaybackRate::SampleRate { frequency: 0.0 };
    }
    fn volume(&self) -> Option<Volume> {
        return None;
    }
    fn timbre(&self) -> Option<Timbre> {
        return None;
    }
    fn amplitude(&self) -> f32 {
        if !self.playing() {
            return 0.0;
        }
        match self.volume() {
            Some(Volume::VolumeIndex { index, max }) => return index as f32 / (max + 1) as f32,
            Some(Volume::Linear { level }) => return level,
            None => return 1.0,
        }
    }
}
