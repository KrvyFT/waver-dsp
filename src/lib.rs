//! Real-time `Process` implementations. Must stay allocation-free in `process`.

#![doc = include_str!("../docs/module-interface.md")]

mod nodes;
mod process;

pub use nodes::{Delay, Noise, Output, Scope, Silence, Vcf, Vco, for_kind};
pub use process::{Process, ProcessCtx};
