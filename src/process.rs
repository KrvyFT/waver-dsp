//! Audio-thread process contract.

/// Context for one block. Borrowed buffers; no heap traffic in `process`.
pub struct ProcessCtx<'a> {
    /// Host sample rate in Hz.
    pub sample_rate: f32,
    /// Frames in this call (may be shorter than the engine's nominal block).
    pub block: usize,
    /// Input buses, each `block` samples (mono planar for the stub).
    pub inputs: &'a [&'a [f32]],
    /// Output buses, each `block` samples. Exclusive write.
    pub outputs: &'a mut [&'a mut [f32]],
}

/// A DSP node. `Send` so the engine can move instances into the cpal callback.
///
/// # Real-time
///
/// Implementations must not allocate, take locks, or perform I/O.
pub trait Process: Send {
    /// Render `ctx.block` frames.
    fn process(&mut self, ctx: &mut ProcessCtx<'_>);
}
