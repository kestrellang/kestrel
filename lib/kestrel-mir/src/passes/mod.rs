//! Post-lowering MIR passes.
//!
//! Pure MIR → MIR transformations that operate on a completed `MirModule`
//! without any ECS/world access.
//!
//! - **Layout**: compute struct sizes, field offsets, alignment, drop order
//! - **Thunk**: scan for `ApplyPartial`, generate/deduplicate thunk wrappers
//! - **Deinit**: analyze liveness, insert `Deinit`/`DeinitIf` for non-copyable values

mod deinit;
mod expand_deinit;
mod layout;
mod thunk;
pub mod verify;

pub use deinit::run_deinit_pass;
pub use expand_deinit::run_expand_deinit_pass;
pub use layout::run_layout_pass;
pub use thunk::run_thunk_pass;
pub use verify::{VerifyResult, verify};
