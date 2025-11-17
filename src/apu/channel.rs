use std::fmt::Debug;

/// Common interface implemented by all five NES APU channels.
///
/// Each channel handles its own register decoding and internal state
/// (envelope, sweep, timers, etc.). The APU core clocks the channels
/// and mixes their raw outputs into an audio sample stream.
pub trait Channel: Debug {
    /// Write to one of the channel specific registers.
    ///
    /// The register index is zero-based relative to the first register
    /// of that channel (e.g. pulse 1 register 0 corresponds to $4000).
    fn write_register(&mut self, register: usize, value: u8);

    /// Enable or disable the channel. Disabling immediately silences
    /// the channel and clears any length counter state.
    fn set_enabled(&mut self, enabled: bool);

    /// A single APU timer tick.
    ///
    /// Channels that rely on DMA (only DMC) may request a sample fetch
    /// by returning the CPU address that needs to be read. All other
    /// channels simply return `None`.
    fn clock_timer(&mut self) -> Option<u16>;

    /// Clock operations tied to the quarter-frame sequencer.
    fn clock_quarter_frame(&mut self);

    /// Clock operations tied to the half-frame sequencer.
    fn clock_half_frame(&mut self);

    /// Current raw output level of the channel as defined on
    /// https://www.nesdev.org/wiki/APU_Mixer. Pulse/Noise outputs are
    /// 0-15, triangle is 0-15, and DMC is 0-127.
    fn output(&self) -> f32;

    /// Whether the channel is still active (e.g. has a running length
    /// counter or buffered DMC bytes). Used by $4015 reads.
    fn active(&self) -> bool;

    /// Returns `true` when the channel has raised an IRQ (frame counter
    /// and DMC). The default implementation covers the channels that
    /// never assert IRQs.
    fn irq_flag(&self) -> bool {
        false
    }

    /// Clear the IRQ latch if the channel supports IRQs.
    fn clear_irq(&mut self) {}

    /// Provide a fetched sample byte. Only used by DMC.
    fn provide_sample(&mut self, _value: u8) {}
}
