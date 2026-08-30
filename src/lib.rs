//! Real-time `Process` implementations. Must stay allocation-free in `process`.

mod nodes;
mod process;

pub use nodes::{Delay, Output, Vco, for_kind, Silence};
pub use process::{Process, ProcessCtx};
