//! One-block delay line inserted by the compiler to break feedback loops.

use crate::{Process, ProcessCtx};

use super::output::MAX_BLOCK;

/// Delays the input by exactly one processing block.
pub struct Delay {
    prev: [f32; MAX_BLOCK],
}

impl Default for Delay {
    fn default() -> Self {
        Self::new()
    }
}

impl Delay {
    /// Zeroed delay buffer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            prev: [0.0; MAX_BLOCK],
        }
    }
}

impl Process for Delay {
    fn process(&mut self, ctx: &mut ProcessCtx<'_>) {
        let n = ctx.block.min(MAX_BLOCK);
        let out = &mut ctx.outputs[0];
        debug_assert!(out.len() >= n);

        if ctx.inputs.is_empty() {
            out[..n].copy_from_slice(&self.prev[..n]);
            self.prev[..n].fill(0.0);
            return;
        }

        let input = ctx.inputs[0];
        debug_assert!(input.len() >= n);
        out[..n].copy_from_slice(&self.prev[..n]);
        self.prev[..n].copy_from_slice(&input[..n]);
    }
}

#[cfg(test)]
mod tests {
    use super::Delay;
    use crate::{Process, ProcessCtx};

    #[test]
    fn delays_by_one_block() {
        let mut delay = Delay::new();
        let input_a = [1.0f32; 4];
        let mut out_a = [0.0f32; 4];
        {
            let mut outputs: [&mut [f32]; 1] = [&mut out_a];
            let mut ctx = ProcessCtx {
                sample_rate: 48_000.0,
                block: 4,
                inputs: &[&input_a],
                outputs: &mut outputs,
            };
            delay.process(&mut ctx);
        }
        assert!(out_a.iter().all(|s| *s == 0.0));

        let input_b = [2.0f32; 4];
        let input_b_slice: &[f32] = &input_b;
        {
            let mut outputs: [&mut [f32]; 1] = [&mut out_a];
            let mut ctx = ProcessCtx {
                sample_rate: 48_000.0,
                block: 4,
                inputs: &[input_b_slice],
                outputs: &mut outputs,
            };
            delay.process(&mut ctx);
        }
        assert!(out_a.iter().all(|s| (*s - 1.0).abs() < f32::EPSILON));
    }
}
