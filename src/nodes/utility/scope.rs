//! Display-only sink: mirrors its input into a shared GUI monitor tap.

use std::sync::Arc;

use waver_core::ScopeTap;

use crate::{Process, ProcessCtx};

/// One input, no audio output: the host hands the tap created for a `monitors` kind.
pub struct Scope {
    tap: Arc<ScopeTap>,
}

impl Scope {
    /// Bind to the tap `ParamRegistry` created for this node.
    #[must_use]
    pub fn new(tap: Arc<ScopeTap>) -> Self {
        Self { tap }
    }
}

impl Process for Scope {
    fn process(&mut self, ctx: &mut ProcessCtx<'_>) {
        let Some(input) = ctx.inputs.first() else {
            // No input bus declared: keep the previous trace.
            return;
        };
        let frames = ctx.block.min(input.len());
        for &sample in &input[..frames] {
            self.tap.push(if sample.is_finite() { sample } else { 0.0 });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use waver_core::ScopeTap;

    use super::Scope;
    use crate::{Process, ProcessCtx};

    fn render(scope: &mut Scope, inputs: &[&[f32]], block: usize) {
        let mut outputs: [&mut [f32]; 0] = [];
        scope.process(&mut ProcessCtx {
            sample_rate: 48_000.0,
            block,
            inputs,
            outputs: &mut outputs,
        });
    }

    fn ramp(count: usize) -> Vec<f32> {
        (0..count).map(|step| step as f32 * 0.01).collect()
    }

    #[test]
    fn captures_the_whole_block() {
        let tap = Arc::new(ScopeTap::new());
        let mut scope = Scope::new(Arc::clone(&tap));
        let signal = ramp(64);
        render(&mut scope, &[&signal], 64);

        let mut out = [f32::NAN; 64];
        assert_eq!(tap.snapshot(&mut out), 64);
        assert_eq!(out, signal.as_slice());
    }

    #[test]
    fn short_block_only_captures_valid_frames() {
        let tap = Arc::new(ScopeTap::new());
        let mut scope = Scope::new(Arc::clone(&tap));
        let signal = ramp(64);
        render(&mut scope, &[&signal], 17);

        let mut out = [f32::NAN; 64];
        assert_eq!(tap.snapshot(&mut out), 17);
        assert_eq!(&out[..17], &signal[..17]);
        assert!(out[17..].iter().all(|sample| sample.is_nan()));
    }

    #[test]
    fn non_finite_samples_are_stored_as_zero() {
        let tap = Arc::new(ScopeTap::new());
        let mut scope = Scope::new(Arc::clone(&tap));
        let signal = [0.5, f32::NAN, f32::INFINITY, -0.25];
        render(&mut scope, &[&signal], 4);

        let mut out = [f32::NAN; 4];
        assert_eq!(tap.snapshot(&mut out), 4);
        assert_eq!(out, [0.5, 0.0, 0.0, -0.25]);
    }

    #[test]
    fn missing_input_bus_keeps_the_previous_trace() {
        let tap = Arc::new(ScopeTap::new());
        let mut scope = Scope::new(Arc::clone(&tap));
        let signal = ramp(4);
        render(&mut scope, &[&signal], 4);
        render(&mut scope, &[], 4);

        let mut out = [f32::NAN; 8];
        assert_eq!(tap.snapshot(&mut out), 4);
        assert_eq!(&out[..4], signal.as_slice());
    }
}
