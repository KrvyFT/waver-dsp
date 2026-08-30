//! Built-in nodes.

mod delay;
mod output;
mod silence;
mod vco;

pub use delay::Delay;
pub use output::Output;
pub use silence::Silence;
pub use vco::Vco;

use waver_core::{NodeId, NodeKind, ParamId, ParamRegistry};

use crate::Process;

/// Map an IR kind to a live processor. Compile-thread only (may allocate).
pub fn for_kind(
    kind: NodeKind,
    node: NodeId,
    params: &ParamRegistry,
) -> Option<Box<dyn Process>> {
    match kind {
        NodeKind::Silence => Some(Box::new(Silence)),
        NodeKind::Vco => {
            let freq = params.get(node, ParamId::new(0))?;
            let amp = params.get(node, ParamId::new(1))?;
            let wave = params.get(node, ParamId::new(2))?;
            Some(Box::new(Vco::with_params(freq, amp, wave)))
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
        let out = graph.insert(NodeKind::Output);
        let schedule = graph.compile().expect("compile");
        let params = ParamRegistry::with_defaults(&schedule);

        assert!(for_kind(NodeKind::Silence, vco, &params).is_some());
        assert!(for_kind(NodeKind::Vco, vco, &params).is_some());
        assert!(for_kind(NodeKind::Output, out, &params).is_some());
        assert!(for_kind(NodeKind::Delay, out, &params).is_some());
        assert!(for_kind(NodeKind::Vcf, vco, &params).is_none());
    }
}
