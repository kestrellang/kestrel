//! Post-lowering MIR passes.
//!
//! Pure MIR → MIR transformations that operate on a completed `MirModule`
//! without any ECS/world access.
//!
//! - **Layout**: compute struct sizes, field offsets, alignment, drop order
//! - **Thunk**: scan for `ApplyPartial`, generate/deduplicate thunk wrappers
//! - **Drop elaboration**: dataflow-based destructor insertion for non-copyable values

pub(crate) mod drop_elaboration;
mod layout;
mod thunk;
pub mod verify;

pub use drop_elaboration::run_drop_elaboration;
pub use layout::run_layout_pass;
pub use thunk::run_thunk_pass;
pub use verify::{VerifyResult, verify};
