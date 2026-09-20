//! Built-in nodes, grouped by [`waver_core::ModuleFamily`].

mod io;
mod oscillator;
mod utility;

pub use io::{MAX_BLOCK, Output};
pub use oscillator::{Noise, Vco};
pub use utility::{Delay, Silence};

use waver_core::{NodeId, NodeKind, ParamId, ParamRegistry};

use crate::Process;

/// Map an IR kind to a live processor. May allocate.
///
/// The current engine calls this from `rebuild` inside the audio callback when
/// applying `SwapSchedule`; construction is not yet a real-time-safe path.
pub fn for_kind(kind: NodeKind, node: NodeId, params: &ParamRegistry) -> Option<Box<dyn Process>> {
    match kind {
        NodeKind::Silence => Some(Box::new(Silence)),
        NodeKind::Vco => {
            let freq = params.get(node, ParamId::new(0))?;
            let amp = params.get(node, ParamId::new(1))?;
            let wave = params.get(node, ParamId::new(2))?;
            Some(Box::new(Vco::with_params(freq, amp, wave)))
        }
        NodeKind::Noise => {
            let amp = params.get(node, ParamId::new(0))?;
            Some(Box::new(Noise::with_seed(amp, node.raw().wrapping_add(1))))
        }
        NodeKind::Output => Some(Box::new(Output::new())),
        NodeKind::Delay => Some(Box::new(Delay::new())),
        NodeKind::Vcf | NodeKind::Vca | NodeKind::Adsr | NodeKind::Lfo | NodeKind::Mixer => None,
    }
}

#[cfg(test)]
mod tests {
    use super::for_kind;
    use waver_core::{Graph, NodeKind, ParamRegistry};

    #[test]
    fn core_kinds_instantiate() {
        let mut graph = Graph::new();
        let vco = graph.insert(NodeKind::Vco);
        let noise = graph.insert(NodeKind::Noise);
        let out = graph.insert(NodeKind::Output);
        let schedule = graph.compile().expect("compile");
        let params = ParamRegistry::with_defaults(&schedule);

        assert!(for_kind(NodeKind::Silence, vco, &params).is_some());
        assert!(for_kind(NodeKind::Vco, vco, &params).is_some());
        assert!(for_kind(NodeKind::Noise, noise, &params).is_some());
        assert!(for_kind(NodeKind::Output, out, &params).is_some());
        assert!(for_kind(NodeKind::Delay, out, &params).is_some());
        assert!(for_kind(NodeKind::Vcf, vco, &params).is_none());
    }
}

#[cfg(test)]
mod contract_tests {
    use super::for_kind;
    use crate::ProcessCtx;
    use waver_core::{Graph, MODULE_CATALOG, NodeKind, ParamId, ParamRegistry};

    #[test]
    fn catalog_availability_matches_factory() {
        for desc in MODULE_CATALOG {
            let mut graph = Graph::new();
            let node = graph.insert(desc.kind);
            let params = ParamRegistry::with_defaults(&graph.compile().unwrap());
            assert_eq!(
                for_kind(desc.kind, node, &params).is_some(),
                desc.addable,
                "{:?}",
                desc.kind
            );
        }
    }

    #[test]
    fn noise_instances_have_distinct_streams_and_live_parameters() {
        let mut graph = Graph::new();
        let a = graph.insert(NodeKind::Noise);
        let b = graph.insert(NodeKind::Noise);
        let params = ParamRegistry::with_defaults(&graph.compile().unwrap());
        let mut first = for_kind(NodeKind::Noise, a, &params).unwrap();
        let mut second = for_kind(NodeKind::Noise, b, &params).unwrap();
        let render = |processor: &mut dyn crate::Process| {
            let mut output = [42.0; 64];
            processor.process(&mut ProcessCtx {
                sample_rate: 48_000.0,
                block: 17,
                inputs: &[],
                outputs: &mut [&mut output],
            });
            assert!(output[17..].iter().all(|sample| *sample == 42.0));
            output
        };
        let samples_a = render(first.as_mut());
        let samples_b = render(second.as_mut());
        assert_ne!(samples_a[..17], samples_b[..17]);
        assert!(
            samples_a[..17]
                .iter()
                .chain(&samples_b[..17])
                .all(|sample| sample.is_finite() && sample.abs() <= 0.2)
        );
        params.get(a, ParamId::new(0)).unwrap().set(0.0);
        assert!(
            render(first.as_mut())[..17]
                .iter()
                .all(|sample| *sample == 0.0)
        );
        assert!(
            render(second.as_mut())[..17]
                .iter()
                .any(|sample| *sample != 0.0)
        );
    }

    #[test]
    fn parameterized_factories_reject_missing_cells() {
        let empty = ParamRegistry::new();
        for kind in [NodeKind::Noise, NodeKind::Vco] {
            assert!(for_kind(kind, waver_core::NodeId::new(0), &empty).is_none());
        }
    }
}
