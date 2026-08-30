use waver_core::ParamCell;

use crate::{Process, ProcessCtx};

const TAU: f32 = std::f32::consts::TAU;

pub struct Vco {
    /// 当前相位累加器，范围 [0, 1)，每帧按 freq/sample_rate 递增。
    phase: f32,
    /// 基频 (Hz)。GUI 侧通过 `ParamCell::set` 写入，音频线程每 block 读取。
    freq: ParamCell,
    /// 输出振幅，线性标量 [0, 1]。
    amp: ParamCell,
    /// 波形选择。0 = sine, 1 = saw, 2 = square, 3 = triangle（约定值，取整后 match）。
    wave: ParamCell,
}

impl Vco {
    pub fn new(freq_hz: f32, amp: f32, wave: f32) -> Self {
        Self {
            phase: 0.0,
            freq: ParamCell::new(freq_hz),
            amp: ParamCell::new(amp),
            wave: ParamCell::new(wave),
        }
    }

    #[inline]
    fn osc_sample(phase: f32, wave: f32) -> f32 {
        match wave as u32 {
            1 => {
                // 锯齿：phase ∈ [0,1) → [-1, 1)
                2.0 * phase - 1.0
            }
            2 => {
                // 方波
                if phase < 0.5 { 1.0 } else { -1.0 }
            }
            _ => {
                // 默认正弦
                (TAU * phase).sin()
            }
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
        // VCO 只有 1 路 mono 输出 → outputs[0]
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
