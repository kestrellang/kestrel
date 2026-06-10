//! Type inference errors.
//!
//! Each variant maps to a user-facing diagnostic. Every `TyKind::Error`
//! in the system has a corresponding `InferError` (ErrorGuaranteed pattern).

use kestrel_ast_builder::Vis;
use kestrel_hecs::Entity;
use kestrel_span::Span;

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
        visibility: Vis,
        span: Span,
    },

    /// Member exists but is static — cannot be accessed on an instance.
    MemberIsStatic {
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

    /// E491: a ref-returning function used as a first-class value (captured,
    /// stored, passed) — the ret_borrow ABI is not expressible in function
    /// types, so this would be a silent-miscompile backdoor.
    RefFunctionAsValue { span: Span },

    /// E492: a reference leaked into a generic type argument via inference
    /// (e.g. `[box.peek()]` inferring `Array[&T]`). Refs are second-class;
    /// bind the value first (`let x = ...`) to store the decayed copy.
    RefInTypeArgument { span: Span },

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
    InstanceMethodAsStatic { name: String, span: Span },

    /// Type parameter used as a standalone value (e.g., `let x = T`).
    TypeParamAsValue { span: Span },

    /// Wrong number of type arguments (e.g., `identity[Int, String](42)` on a 1-param generic).
    TypeArgCountMismatch {
        expected: usize,
        got: usize,
        span: Span,
    },

    /// No overload matches the call's labels/arity (e.g., enum case with wrong labels).
    NoMatchingOverload { name: String, span: Span },

    /// Memberwise init call has wrong number of arguments for the struct's fields.
    /// Emitted for `Point(x: 1)` when `Point` has two fields.
    MemberwiseInitArity {
        struct_name: String,
        expected: usize,
        got: usize,
        span: Span,
    },

    /// Memberwise init call has a wrong label for a field.
    /// Emitted for `Point(a: 1, b: 2)` when `Point` has fields `x`, `y`.
    MemberwiseInitLabel {
        struct_name: String,
        expected: String,
        got: Option<String>,
        span: Span,
    },

    /// Implicit `it` parameter used in a context expecting != 1 parameter.
    ItWrongArity { expected: usize, span: Span },

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

    /// A generic type parameter at a call/ref site couldn't be inferred —
    /// no argument, receiver, or context constrained it. Typically happens
    /// when a type parameter only appears in an unused branch of the return
    /// type (e.g. `E` in `Result[T, E]` when the closure only returns `.Ok`).
    ///
    /// Instead of silently defaulting to `Never` (lib1's behavior), we
    /// require the user to annotate — either at the call (`f[T, U](...)`)
    /// or at the binding (`let x: Result[T, U] = f(...)`).
    UnresolvedTypeParam {
        /// The TypeParameter entity whose name is shown in the diagnostic.
        param: Entity,
        /// The call site's span — used as the diagnostic's primary label.
        span: Span,
    },

    /// An expression or local's type stayed fully unresolved through solving —
    /// no constraint pinned it down, and it isn't a generic-call type arg
    /// (which `UnresolvedTypeParam` handles). Points at the expression / local
    /// binding so the user can add an annotation.
    ///
    /// Reported by the phase-4.5 sweep in `solver::report_unresolved_slots`.
    /// Before this existed, the slot silently became `MirTy::Error` in
    /// downstream lowering and triggered a Cranelift type-mismatch panic.
    CannotInferType { span: Span },

    /// Tuple-index access (`x.0`) on a receiver that isn't a tuple type.
    TupleIndexOnNonTuple {
        receiver: TyVar,
        index: usize,
        span: Span,
    },

    /// Tuple-index access where the index is beyond the tuple's arity.
    TupleIndexOutOfBounds {
        arity: usize,
        index: usize,
        span: Span,
    },

    /// Member access on a primitive/intrinsic type that isn't a known method.
    MemberAccessOnPrimitive {
        receiver: TyVar,
        name: String,
        span: Span,
    },

    /// Referencing a known primitive method without calling it.
    /// `x.toString` (when the user meant `x.toString()`) — primitive methods
    /// cannot be used as first-class values.
    MethodNotCalled {
        receiver: TyVar,
        method: String,
        span: Span,
    },

    /// Circular opaque type inference: the concrete type behind `some P`
    /// is itself another `some P` from a mutually recursive call, so no
    /// concrete type can be determined.
    CircularOpaqueReturn { span: Span },

    /// A `mutating` closure was passed where a non-mutating (`Borrow`/
    /// `Consuming`) closure parameter is expected — the callee never lends a
    /// mutable place, so the closure's write access can't be honored (#106).
    ConventionMismatch { span: Span },
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
            | Self::MemberIsStatic { span, .. }
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
            | Self::MemberwiseInitArity { span, .. }
            | Self::MemberwiseInitLabel { span, .. }
            | Self::ItWrongArity { span, .. }
            | Self::LiteralNotAccepted { span, .. }
            | Self::UnresolvedTypeParam { span, .. }
            | Self::CannotInferType { span, .. }
            | Self::TupleIndexOnNonTuple { span, .. }
            | Self::TupleIndexOutOfBounds { span, .. }
            | Self::MemberAccessOnPrimitive { span, .. }
            | Self::MethodNotCalled { span, .. }
            | Self::CircularOpaqueReturn { span }
            | Self::RefFunctionAsValue { span }
            | Self::RefInTypeArgument { span }
            | Self::ConventionMismatch { span } => span,
        }
    }
}
