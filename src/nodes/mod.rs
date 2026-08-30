//! Built-in nodes. Only [`Silence`] exists in the skeleton.

mod silence;
mod vco;

pub use silence::Silence;
pub use vco::Vco;

use waver_core::NodeKind;

use crate::Process;

/// Map an IR kind to a live processor. Compile-thread only (may allocate later).
pub fn for_kind(kind: NodeKind) -> Option<Box<dyn Process>> {
    match kind {
        NodeKind::Silence => Some(Box::new(Silence)),
        NodeKind::Vco => Some(Box::new(Vco::new(440.0, 0.5, 0.0))),
        NodeKind::Vcf
        | NodeKind::Vca
        | NodeKind::Adsr
        | NodeKind::Lfo
        | NodeKind::Mixer
        | NodeKind::Output
        | NodeKind::Delay => None,
    }
}

#[cfg(test)]
mod tests {
    use super::for_kind;
    use waver_core::NodeKind;

    #[test]
    fn only_silence_kind_instantiates() {
        assert!(for_kind(NodeKind::Silence).is_some());
        assert!(for_kind(NodeKind::Vco).is_none());
    }
}
