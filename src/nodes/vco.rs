use std::sync::Arc;

use waver_core::ParamCell;

use crate::{Process, ProcessCtx};

const TAU: f32 = std::f32::consts::TAU;

pub struct Vco {
    /// 当前相位累加器，范围 [0, 1)，每帧按 freq/sample_rate 递增。
    phase: f32,
    freq: Arc<ParamCell>,
    amp: Arc<ParamCell>,
    wave: Arc<ParamCell>,
}

impl Vco {
    /// 使用共享参数单元构造 VCO。
    pub fn with_params(freq: Arc<ParamCell>, amp: Arc<ParamCell>, wave: Arc<ParamCell>) -> Self {
        Self {
            phase: 0.0,
            freq,
            amp,
            wave,
        }
    }

    #[cfg(test)]
    fn set_phase(&mut self, phase: f32) {
        self.phase = phase;
    }

    #[cfg(test)]
    fn phase(&self) -> f32 {
        self.phase
    }

    #[inline]
    fn osc_sample(phase: f32, wave: f32) -> f32 {
        match wave as u32 {
            1 => 2.0 * phase - 1.0,
            2 => {
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            3 => 1.0 - 4.0 * (phase - 0.5).abs(),
            _ => (TAU * phase).sin(),
        }
    }
}

impl Process for Vco {
    fn process(&mut self, ctx: &mut ProcessCtx<'_>) {
        let sr = ctx.sample_rate;
        let n = ctx.block;
        let freq = self.freq.value().clamp(20.0, 20_000.0);
        let amp = self.amp.value().clamp(0.0, 1.0);
        let wave = self.wave.value();
        let phase_inc = freq / sr;
        let out = &mut ctx.outputs[0];
        debug_assert!(out.len() >= n);
        for i in 0..n {
            out[i] = Self::osc_sample(self.phase, wave) * amp;
            self.phase += phase_inc;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use waver_core::ParamCell;

    use super::Vco;
    use crate::{Process, ProcessCtx};

    fn run_block(vco: &mut Vco, n: usize) -> Vec<f32> {
        let mut out = vec![0.0f32; n];
        let mut outputs: [&mut [f32]; 1] = [&mut out];
        let mut ctx = ProcessCtx {
            sample_rate: 48_000.0,
            block: n,
            inputs: &[],
            outputs: &mut outputs,
        };
        vco.process(&mut ctx);
        out
    }

    #[test]
    fn sine_at_zero_phase() {
        let mut vco = Vco::with_params(
            Arc::new(ParamCell::new(440.0)),
            Arc::new(ParamCell::new(1.0)),
            Arc::new(ParamCell::new(0.0)),
        );
        let samples = run_block(&mut vco, 1);
        assert!(samples[0].abs() < 1e-5);
    }

    #[test]
    fn saw_mid_phase() {
        let mut vco = Vco::with_params(
            Arc::new(ParamCell::new(440.0)),
            Arc::new(ParamCell::new(1.0)),
            Arc::new(ParamCell::new(1.0)),
        );
        vco.set_phase(0.75);
        let samples = run_block(&mut vco, 1);
        assert!((samples[0] - 0.5).abs() < 1e-5);
    }

    #[test]
    fn square_high_low() {
        let mut vco = Vco::with_params(
            Arc::new(ParamCell::new(440.0)),
            Arc::new(ParamCell::new(1.0)),
            Arc::new(ParamCell::new(2.0)),
        );
        vco.set_phase(0.25);
        assert!((run_block(&mut vco, 1)[0] - 1.0).abs() < 1e-5);
        vco.set_phase(0.75);
        assert!((run_block(&mut vco, 1)[0] + 1.0).abs() < 1e-5);
    }

    #[test]
    fn triangle_peak() {
        let mut vco = Vco::with_params(
            Arc::new(ParamCell::new(440.0)),
            Arc::new(ParamCell::new(1.0)),
            Arc::new(ParamCell::new(3.0)),
        );
        vco.set_phase(0.5);
        assert!((run_block(&mut vco, 1)[0] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn phase_wraps() {
        let mut vco = Vco::with_params(
            Arc::new(ParamCell::new(24_000.0)),
            Arc::new(ParamCell::new(1.0)),
            Arc::new(ParamCell::new(0.0)),
        );
        vco.set_phase(0.99);
        run_block(&mut vco, 1);
        assert!(vco.phase() < 0.6);
    }
}
