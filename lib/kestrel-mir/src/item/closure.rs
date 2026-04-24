//! Closure information — first-class structural relationship between
//! env struct, call function, and captures.

use crate::id::{FunctionId, StructId};
use crate::ty::MirTy;

/// Closure metadata — the relationship between env struct, call function,
/// and captured values.
#[derive(Debug, Clone)]
pub struct ClosureInfo {
    /// The generated environment struct holding captured values.
    pub env_struct: StructId,
    /// The generated call function (closure body).
    pub call_function: FunctionId,
    /// Captured values and how they're captured.
    pub captures: Vec<CaptureInfo>,
}

/// A single captured value in a closure.
#[derive(Debug, Clone)]
pub struct CaptureInfo {
    /// Name of the captured variable.
    pub name: String,
    /// Type of the captured value.
    pub ty: MirTy,
    /// How the value is captured.
    pub mode: CaptureMode,
}

impl CaptureInfo {
    pub fn new(name: impl Into<String>, ty: MirTy, mode: CaptureMode) -> Self {
        Self {
            name: name.into(),
            ty,
            mode,
        }
    }
}

/// How a value is captured by a closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaptureMode {
    /// Captured by immutable reference.
    ByRef,
    /// Captured by mutable reference.
    ByMutRef,
    /// Captured by move (ownership transferred).
    ByMove,
    /// Captured by copy.
    ByCopy,
}
