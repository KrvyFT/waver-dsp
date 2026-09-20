//! White-noise oscillator — standard demo of a second module under one family.

use std::sync::Arc;

use waver_core::ParamCell;

use crate::{Process, ProcessCtx};

/// xorshift32 white noise scaled by amplitude.
pub struct Noise {
    state: u32,
    amp: Arc<ParamCell>,
}

impl Noise {
    /// Shared amplitude cell (param 0).
    pub fn with_params(amp: Arc<ParamCell>) -> Self {
        Self::with_seed(amp, 0xA5_A5_F1_37)
    }

    /// Reproducible stream. Distinct non-zero seeds produce distinct streams.
    /// Zero is replaced because xorshift32 would otherwise remain silent.
    pub fn with_seed(amp: Arc<ParamCell>, seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0xA5_A5_F1_37 } else { seed },
            amp,
        }
    }

    #[inline]
    fn next_unit(&mut self) -> f32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        // Map u32 to [-1, 1].
        (x as f32 / (u32::MAX as f32)) * 2.0 - 1.0
    }
}

impl Process for Noise {
    fn process(&mut self, ctx: &mut ProcessCtx<'_>) {
        let n = ctx.block;
        let amp = self.amp.value().clamp(0.0, 1.0);
        let out = &mut ctx.outputs[0];
        debug_assert!(out.len() >= n);
        for i in 0..n {
            out[i] = self.next_unit() * amp;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use waver_core::ParamCell;

    use super::Noise;
    use crate::{Process, ProcessCtx};

    #[test]
    fn produces_non_silent_when_amp_positive() {
        let mut noise = Noise::with_params(Arc::new(ParamCell::new(0.5)));
        let mut out = [0.0f32; 64];
        {
            let mut outputs: [&mut [f32]; 1] = [&mut out];
            let mut ctx = ProcessCtx {
                sample_rate: 48_000.0,
                block: 64,
                inputs: &[],
                outputs: &mut outputs,
            };
            noise.process(&mut ctx);
        }
        let energy: f32 = out.iter().map(|s| s * s).sum();
        assert!(energy > 0.01, "expected noise energy, got {energy}");
    }

    #[test]
    fn zero_amp_is_silent() {
        let mut noise = Noise::with_params(Arc::new(ParamCell::new(0.0)));
        let mut out = [1.0f32; 16];
        {
            let mut outputs: [&mut [f32]; 1] = [&mut out];
            let mut ctx = ProcessCtx {
                sample_rate: 48_000.0,
                block: 16,
                inputs: &[],
                outputs: &mut outputs,
            };
            noise.process(&mut ctx);
        }
        assert!(out.iter().all(|s| *s == 0.0));
    }
}
