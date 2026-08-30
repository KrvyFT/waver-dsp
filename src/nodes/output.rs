//! Master output sink: copies input bus into an engine-readable buffer.

use crate::{Process, ProcessCtx};

/// Must match [`waver_engine::BLOCK`].
pub const MAX_BLOCK: usize = 64;

/// Terminal node with one input and no graph outputs.
pub struct Output {
    master: [f32; MAX_BLOCK],
    len: usize,
}

impl Default for Output {
    fn default() -> Self {
        Self::new()
    }
}

impl Output {
    /// Zeroed master buffer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            master: [0.0; MAX_BLOCK],
            len: 0,
        }
    }

    /// Last rendered mono block copied from the input jack.
    #[must_use]
    pub fn master_slice(&self) -> &[f32] {
        &self.master[..self.len]
    }
}

impl Process for Output {
    fn process(&mut self, ctx: &mut ProcessCtx<'_>) {
        let n = ctx.block.min(MAX_BLOCK);
        self.len = n;
        if ctx.inputs.is_empty() {
            self.master[..n].fill(0.0);
            return;
        }
        let input = ctx.inputs[0];
        debug_assert!(input.len() >= n);
        self.master[..n].copy_from_slice(&input[..n]);
    }
}

#[cfg(test)]
mod tests {
    use super::Output;
    use crate::{Process, ProcessCtx};

    #[test]
    fn copies_input_to_master() {
        let mut output = Output::new();
        let input = [0.25f32; 8];
        let mut dummy_out = [0.0f32; 8];
        let mut outputs: [&mut [f32]; 1] = [&mut dummy_out];
        let mut ctx = ProcessCtx {
            sample_rate: 48_000.0,
            block: 8,
            inputs: &[&input],
            outputs: &mut outputs,
        };
        output.process(&mut ctx);
        assert!(output.master_slice().iter().all(|s| (*s - 0.25).abs() < f32::EPSILON));
    }
}
