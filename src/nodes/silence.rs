//! Output-zeroing stub used until real modules are wired into the schedule.

use crate::{Process, ProcessCtx};

/// Writes zeros to every output bus.
#[derive(Clone, Copy, Debug, Default)]
pub struct Silence;

impl Process for Silence {
    fn process(&mut self, ctx: &mut ProcessCtx<'_>) {
        for out in ctx.outputs.iter_mut() {
            out.fill(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Silence;
    use crate::{Process, ProcessCtx};

    #[test]
    fn silence_clears_outputs() {
        let mut silence = Silence;
        let mut out_a = [1.0f32; 8];
        let mut out_b = [1.0f32; 8];
        let mut outputs: [&mut [f32]; 2] = [&mut out_a, &mut out_b];
        let mut ctx = ProcessCtx {
            sample_rate: 48_000.0,
            block: 8,
            inputs: &[],
            outputs: &mut outputs,
        };
        silence.process(&mut ctx);
        assert!(out_a.iter().all(|s| *s == 0.0));
        assert!(out_b.iter().all(|s| *s == 0.0));
    }
}
