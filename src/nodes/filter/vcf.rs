//! Two-pole TPT state-variable low-pass filter.

use std::sync::Arc;

use waver_core::ParamCell;

use crate::{Process, ProcessCtx};

/// 截止下限（Hz）。
const MIN_CUTOFF_HZ: f32 = 20.0;
/// 截止上限（Hz）。
const MAX_CUTOFF_HZ: f32 = 20_000.0;
/// ±1.0 的 CV 折算成多少个八度。
const CV_OCTAVES: f32 = 4.0;
/// 截止平滑的时间常数（秒）。
const SMOOTH_SECONDS: f32 = 0.01;
/// 阻尼下限：`k → 0` 会自激并可能发散。
const MIN_DAMPING: f32 = 0.05;
/// 截止非有限值时的回退（与 catalog 默认值一致）。
const FALLBACK_CUTOFF_HZ: f32 = 2_500.0;
/// 共振非有限值时的回退。
const FALLBACK_RESONANCE: f32 = 0.2;

/// 二阶 TPT（梯形积分）状态变量低通。
///
/// - 参数 0：截止频率（Hz，`MIN_CUTOFF_HZ..=MAX_CUTOFF_HZ`），对数平滑。
/// - 参数 1：归一化共振（阻尼 `k = 2 − 2r`，下限 `MIN_DAMPING`）。
/// - 输入 0：音频；输入 1：CV，按 ±`CV_OCTAVES` 个八度调制截止（每块取一个样本）。
///
/// 系数按块更新（控制率），块内固定；因此把音频速率源接到 CV 会得到块率调制。
/// 采样率偏低时截止被压到 `0.45·sr` 以下，避免 `tan` 逼近奈奎斯特处的极点。
pub struct Vcf {
    cutoff: Arc<ParamCell>,
    resonance: Arc<ParamCell>,
    /// TPT 积分器状态。
    ic1eq: f32,
    ic2eq: f32,
    /// 平滑后的截止，存 `log10(Hz)`（控制率）。
    log_hz: f32,
    /// 本块阻尼 `k = 1/Q`。
    damping: f32,
}

impl Vcf {
    /// 使用共享参数单元构造低通滤波器。
    pub fn with_params(cutoff: Arc<ParamCell>, resonance: Arc<ParamCell>) -> Self {
        let hz = sanitize(
            cutoff.value(),
            FALLBACK_CUTOFF_HZ,
            MIN_CUTOFF_HZ,
            MAX_CUTOFF_HZ,
        );
        let resonance_norm = sanitize(resonance.value(), FALLBACK_RESONANCE, 0.0, 1.0);
        Self {
            cutoff,
            resonance,
            ic1eq: 0.0,
            ic2eq: 0.0,
            log_hz: hz.log10(),
            damping: damping_from(resonance_norm),
        }
    }
}

/// 非有限值回退 `fallback`，否则 clamp 到 `min..=max`。
fn sanitize(value: f32, fallback: f32, min: f32, max: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

/// 归一化共振 → 阻尼 `k = 1/Q`。
fn damping_from(resonance: f32) -> f32 {
    (2.0 - 2.0 * resonance).clamp(MIN_DAMPING, 2.0)
}

/// 采样率允许的最高截止：保留 `0.45·sr` 的安全边界，但不超过 `MAX_CUTOFF_HZ`。
/// 结果始终落在 `MIN_CUTOFF_HZ..=MAX_CUTOFF_HZ` 内，后面 `fc` 的 clamp 因此满足 min ≤ max。
fn cutoff_ceiling(sample_rate: f32) -> f32 {
    (0.45 * sample_rate).clamp(MIN_CUTOFF_HZ, MAX_CUTOFF_HZ)
}

impl Process for Vcf {
    fn process(&mut self, ctx: &mut ProcessCtx<'_>) {
        if ctx.outputs.is_empty() {
            return;
        }
        let n = ctx.block;
        let sr = ctx.sample_rate;
        if !sr.is_finite() || sr <= 0.0 {
            return;
        }
        let inputs = ctx.inputs;
        let audio = inputs.first().copied();
        let cv = inputs
            .get(1)
            .and_then(|bus| bus.first().copied())
            .unwrap_or(0.0);

        // 控制率：每块更新一次目标截止与阻尼，块内系数固定。
        let ceiling = cutoff_ceiling(sr);
        let raw_cutoff = self.cutoff.value();
        let target = if raw_cutoff.is_finite() && cv.is_finite() {
            let modulated =
                raw_cutoff.clamp(MIN_CUTOFF_HZ, MAX_CUTOFF_HZ) * (cv * CV_OCTAVES).exp2();
            if modulated.is_finite() {
                modulated.clamp(MIN_CUTOFF_HZ, ceiling).log10()
            } else {
                self.log_hz
            }
        } else {
            self.log_hz
        };
        let alpha = 1.0 - (-(n as f32) / (SMOOTH_SECONDS * sr)).exp();
        self.log_hz += alpha * (target - self.log_hz);
        if !self.log_hz.is_finite() {
            self.log_hz = FALLBACK_CUTOFF_HZ.log10();
        }

        let raw_resonance = self.resonance.value();
        if raw_resonance.is_finite() {
            self.damping = damping_from(raw_resonance.clamp(0.0, 1.0));
        }

        let fc = (self.log_hz * std::f32::consts::LOG2_10)
            .exp2()
            .clamp(MIN_CUTOFF_HZ, ceiling);
        let g = (std::f32::consts::PI * fc / sr).tan();
        let k = self.damping;
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;

        let out = &mut ctx.outputs[0];
        debug_assert!(out.len() >= n);
        let mut ic1eq = self.ic1eq;
        let mut ic2eq = self.ic2eq;
        for i in 0..n {
            let x = audio.and_then(|bus| bus.get(i).copied()).unwrap_or(0.0);
            let v3 = x - ic2eq;
            let v1 = a1 * ic1eq + a2 * v3;
            let v2 = ic2eq + a2 * ic1eq + a3 * v3;
            ic1eq = 2.0 * v1 - ic1eq;
            ic2eq = 2.0 * v2 - ic2eq;
            out[i] = v2;
        }
        if ic1eq.is_finite() && ic2eq.is_finite() {
            self.ic1eq = ic1eq;
            self.ic2eq = ic2eq;
        } else {
            self.ic1eq = 0.0;
            self.ic2eq = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use waver_core::ParamCell;

    use super::{MAX_CUTOFF_HZ, MIN_CUTOFF_HZ, Vcf};
    use crate::{Process, ProcessCtx};

    const SR: f32 = 48_000.0;
    const BLOCK: usize = 64;

    /// 截止以 Hz 给出（catalog 默认 2 500 Hz）。
    fn vcf(cutoff_hz: f32, resonance: f32) -> Vcf {
        Vcf::with_params(
            Arc::new(ParamCell::new(cutoff_hz)),
            Arc::new(ParamCell::new(resonance)),
        )
    }

    fn run(vcf: &mut Vcf, input: &[f32], cv: Option<&[f32]>) -> Vec<f32> {
        let n = input.len();
        let mut out = vec![0.0f32; n];
        let mut buses: Vec<&[f32]> = vec![input];
        if let Some(cv) = cv {
            buses.push(cv);
        }
        let mut outputs: [&mut [f32]; 1] = [&mut out];
        let mut ctx = ProcessCtx {
            sample_rate: SR,
            block: n,
            inputs: &buses,
            outputs: &mut outputs,
        };
        vcf.process(&mut ctx);
        out
    }

    /// 渲染 `blocks` 个 64 帧块，返回最后一块。`cv` 为 Some 时给输入口 1 一条常值总线。
    fn render(
        vcf: &mut Vcf,
        blocks: usize,
        mut signal: impl FnMut(usize) -> f32,
        cv: Option<f32>,
    ) -> Vec<f32> {
        let mut last = vec![0.0f32; BLOCK];
        for block in 0..blocks {
            let input: Vec<f32> = (0..BLOCK).map(|i| signal(block * BLOCK + i)).collect();
            last = match cv {
                Some(value) => {
                    let bus = vec![value; BLOCK];
                    run(vcf, &input, Some(&bus))
                }
                None => run(vcf, &input, None),
            };
        }
        last
    }

    fn peak(samples: &[f32]) -> f32 {
        samples.iter().fold(0.0f32, |m, s| m.max(s.abs()))
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    }

    fn nyquist(i: usize) -> f32 {
        if i % 2 == 0 { 1.0 } else { -1.0 }
    }

    fn tone_1k(i: usize) -> f32 {
        (std::f32::consts::TAU * 1_000.0 * i as f32 / SR).sin()
    }

    #[test]
    fn no_input_bus_is_silent() {
        let mut vcf = vcf(2_500.0, 0.2);
        let mut out = [1.0f32; BLOCK];
        let mut outputs: [&mut [f32]; 1] = [&mut out];
        let mut ctx = ProcessCtx {
            sample_rate: SR,
            block: BLOCK,
            inputs: &[],
            outputs: &mut outputs,
        };
        vcf.process(&mut ctx);
        assert!(out.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn dc_passes_through() {
        let mut vcf = vcf(2_500.0, 0.2);
        let last = render(&mut vcf, 8, |_| 1.0, None);
        assert!((last[BLOCK - 1] - 1.0).abs() < 0.01, "{}", last[BLOCK - 1]);
    }

    #[test]
    fn nyquist_is_attenuated() {
        let mut vcf = vcf(MIN_CUTOFF_HZ, 0.2);
        let last = render(&mut vcf, 8, nyquist, None);
        assert!(peak(&last) < 1e-3, "{}", peak(&last));
    }

    /// 参数 0 是 Hz，不是归一化量：20 kHz 通 1 kHz，200 Hz 挡 1 kHz。
    #[test]
    fn cutoff_reads_in_hertz() {
        let mut open = vcf(MAX_CUTOFF_HZ, 0.2);
        let pass = render(&mut open, 40, tone_1k, None);
        let mut closed = vcf(200.0, 0.2);
        let stop = render(&mut closed, 40, tone_1k, None);
        assert!(
            rms(&pass) > 10.0 * rms(&stop),
            "{} vs {}",
            rms(&pass),
            rms(&stop)
        );
    }

    #[test]
    fn cv_opens_the_filter() {
        let mut closed = vcf(160.0, 0.2);
        let out_closed = render(&mut closed, 20, tone_1k, Some(-1.0));
        let mut open = vcf(160.0, 0.2);
        let out_open = render(&mut open, 20, tone_1k, Some(1.0));
        assert!(
            rms(&out_open) > 5.0 * rms(&out_closed),
            "{} vs {}",
            rms(&out_open),
            rms(&out_closed)
        );
        assert!(
            out_open
                .iter()
                .chain(&out_closed)
                .all(|s| s.is_finite() && s.abs() <= 1.0)
        );
    }

    #[test]
    fn live_parameter_changes_apply_next_block() {
        let cutoff = Arc::new(ParamCell::new(MIN_CUTOFF_HZ));
        let resonance = Arc::new(ParamCell::new(0.2));
        let mut vcf = Vcf::with_params(cutoff.clone(), resonance);
        let before = render(&mut vcf, 8, nyquist, None);
        cutoff.set(MAX_CUTOFF_HZ);
        let after = render(&mut vcf, 8, nyquist, None);
        assert!(
            peak(&after) > 10.0 * peak(&before),
            "{} vs {}",
            peak(&after),
            peak(&before)
        );
    }

    #[test]
    fn non_finite_parameters_do_not_poison_state() {
        let cutoff = Arc::new(ParamCell::new(f32::NAN));
        let resonance = Arc::new(ParamCell::new(f32::INFINITY));
        let mut vcf = Vcf::with_params(cutoff, resonance);
        let last = render(&mut vcf, 10, nyquist, None);
        assert!(last.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
    }

    #[test]
    fn extreme_settings_stay_bounded() {
        let mut vcf = vcf(MAX_CUTOFF_HZ, 1.0);
        let last = render(&mut vcf, 200, tone_1k, None);
        assert!(last.iter().all(|s| s.is_finite() && s.abs() < 5.0));
    }

    #[test]
    fn short_block_leaves_tail_untouched() {
        let mut vcf = vcf(2_500.0, 0.2);
        let input = [1.0f32; 17];
        let mut out = [42.0f32; BLOCK];
        let mut outputs: [&mut [f32]; 1] = [&mut out];
        let mut ctx = ProcessCtx {
            sample_rate: SR,
            block: 17,
            inputs: &[&input],
            outputs: &mut outputs,
        };
        vcf.process(&mut ctx);
        assert!(out[..17].iter().all(|s| s.is_finite()));
        assert!(out[17..].iter().all(|s| *s == 42.0));
    }
}
