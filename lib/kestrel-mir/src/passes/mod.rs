//! Post-lowering MIR passes.
//!
//! Pure MIR → MIR transformations that operate on a completed `MirModule`
//! without any ECS/world access.
//!
//! - **Layout**: compute struct sizes, field offsets, alignment, drop order
//! - **Thunk**: scan for `ApplyPartial`, generate/deduplicate thunk wrappers
//!
//! Drop elaboration is owned by [`kestrel_ownership::drop_elab`]; the
//! verifier (`verify`) enforces that lowering never emits `Drop` / `DropIf`
//! itself.

mod layout;
mod thunk;
pub mod verify;

pub use layout::run_layout_pass;
pub use thunk::run_thunk_pass;
pub use verify::{VerifyResult, VerifyStage, place_type, verify, verify_with_stage};
