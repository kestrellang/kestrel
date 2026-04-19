//! Type inference errors.
//!
//! Each variant maps to a user-facing diagnostic. Every `TyKind::Error`
//! in the system has a corresponding `InferError` (ErrorGuaranteed pattern).

use kestrel_hecs::Entity;
use kestrel_span2::Span;

use crate::ty::{LiteralKind, TyVar};

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
    /// `is_call` distinguishes a method/init lookup (`x.foo(...)`) from a
    /// field/property access (`x.foo`); it drives the diagnostic wording
    /// ("no method '...' on type 'T'" vs "no member '...' on type 'T'").
    NoMember {
        receiver: TyVar,
        name: String,
        is_call: bool,
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

    /// Wrong number of type arguments (e.g., `identity[Int, String](42)` on a 1-param generic).
    TypeArgCountMismatch {
        expected: usize,
        got: usize,
        span: Span,
    },

    /// No overload matches the call's labels/arity (e.g., enum case with wrong labels).
    NoMatchingOverload {
        name: String,
        span: Span,
    },

    /// Implicit `it` parameter used in a context expecting != 1 parameter.
    ItWrongArity {
        expected: usize,
        span: Span,
    },

    /// A literal of a given kind can't be accepted by the target type.
    /// Emitted when an unresolved literal TyVar meets a concrete type that
    /// doesn't conform to the corresponding `ExpressibleBy*Literal` protocol.
    /// Used instead of `DoesNotConform` when the protocol entity isn't
    /// available (e.g., stdlib disabled) — we still know the literal kind
    /// from the TySlot.
    LiteralNotAccepted {
        ty: TyVar,
        literal: LiteralKind,
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
            | Self::TypeParamAsValue { span }
            | Self::TypeArgCountMismatch { span, .. }
            | Self::NoMatchingOverload { span, .. }
            | Self::ItWrongArity { span, .. }
            | Self::LiteralNotAccepted { span, .. } => span,
        }
    }
}
