//! Post-lowering MIR passes.
//!
//! Pure MIR → MIR transformations that operate on a completed `MirModule`
//! without any ECS/world access.
//!
//! - **Layout**: compute struct sizes, field offsets, alignment, drop order
//! - **Thunk**: scan for `ApplyPartial`, generate/deduplicate thunk wrappers
//! - **Drop elaboration**: dataflow-based destructor insertion for non-copyable values
//!
//! Drop elaboration is owned by [`kestrel_ownership::drop_elab`]; the
//! verifier (`verify`) enforces that lowering never emits `Drop` / `DropIf`
//! itself.

mod clone_elaboration;
pub(crate) mod drop_elaboration;
mod layout;
pub mod liveness;
mod thunk;
pub mod verify;

pub use clone_elaboration::run_clone_elaboration;
pub use drop_elaboration::run_drop_elaboration;
pub use layout::run_layout_pass;
pub use liveness::Liveness;
pub use thunk::run_thunk_pass;
pub use verify::{VerifyResult, VerifyStage, place_type, verify, verify_with_stage};
