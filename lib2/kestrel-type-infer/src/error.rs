//! Type inference errors.
//!
//! Each variant maps to a user-facing diagnostic. Every `TyKind::Error`
//! in the system has a corresponding `InferError` (ErrorGuaranteed pattern).

use kestrel_hecs::Entity;
use kestrel_span2::Span;

use crate::ty::TyVar;

/// A type inference error. Accumulated during solving; each produces
/// a `TyKind::Error` TyVar that silently absorbs further constraints.
#[derive(Clone, Debug)]
pub enum InferError {
    /// Types don't match (structural mismatch).
    TypeMismatch {
        expected: TyVar,
        got: TyVar,
        span: Span,
    },

    /// Type doesn't conform to a protocol.
    DoesNotConform {
        ty: TyVar,
        protocol: Entity,
        span: Span,
    },

    /// No member with this name on the receiver type.
    NoMember {
        receiver: TyVar,
        name: String,
        span: Span,
    },

    /// Multiple candidates for a member — ambiguous.
    AmbiguousMember {
        receiver: TyVar,
        name: String,
        span: Span,
    },

    /// Member exists but is not visible from the current context.
    MemberNotVisible {
        receiver: TyVar,
        name: String,
        span: Span,
    },

    /// No associated type with this name on the container.
    NoAssociatedType {
        container: TyVar,
        name: String,
        span: Span,
    },

    /// Infinite type (occurs check failure).
    InfiniteType { span: Span },

    /// Error propagated from HIR (HirExpr::Error, HirPat::Error, etc.)
    FromHir { span: Span },

    /// Implicit member `.name` not found on expected type.
    ImplicitMemberNotFound {
        expected: TyVar,
        name: String,
        span: Span,
    },

    /// Wrong number of arguments in a call.
    ArgCountMismatch {
        expected: usize,
        got: usize,
        span: Span,
    },

    /// Wrong argument label in a call.
    LabelMismatch {
        expected: Option<String>,
        got: Option<String>,
        span: Span,
    },

    /// Instance method called in static context (e.g., `T.instanceMethod()`).
    InstanceMethodAsStatic {
        name: String,
        span: Span,
    },

    /// Type parameter used as a standalone value (e.g., `let x = T`).
    TypeParamAsValue {
        span: Span,
    },
}

impl InferError {
    /// The source span where this error occurred.
    pub fn span(&self) -> &Span {
        match self {
            Self::TypeMismatch { span, .. }
            | Self::DoesNotConform { span, .. }
            | Self::NoMember { span, .. }
            | Self::AmbiguousMember { span, .. }
            | Self::MemberNotVisible { span, .. }
            | Self::NoAssociatedType { span, .. }
            | Self::InfiniteType { span }
            | Self::FromHir { span }
            | Self::ImplicitMemberNotFound { span, .. }
            | Self::ArgCountMismatch { span, .. }
            | Self::LabelMismatch { span, .. }
            | Self::InstanceMethodAsStatic { span, .. }
            | Self::TypeParamAsValue { span } => span,
        }
    }
}
