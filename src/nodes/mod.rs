//! Built-in nodes. Only [`Silence`] exists in the skeleton.

mod silence;

pub use silence::Silence;

use waver_core::NodeKind;

/// Map an IR kind to a live processor. Compile-thread only (may allocate later).
pub fn for_kind(kind: NodeKind) -> Option<Silence> {
    match kind {
        NodeKind::Silence => Some(Silence),
        NodeKind::Vco
        | NodeKind::Vcf
        | NodeKind::Vca
        | NodeKind::Adsr
        | NodeKind::Lfo
        |         NodeKind::Mixer
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
