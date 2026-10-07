//! Type inference errors.
//!
//! Each variant maps to a user-facing diagnostic. Every `TyKind::Error`
//! in the system has a corresponding `InferError` (ErrorGuaranteed pattern).

use kestrel_ast_builder::Vis;
use kestrel_hecs::Entity;
use kestrel_span::Span;

use crate::constraint::Reason;
use crate::ty::{LiteralKind, TyVar};

/// A type inference error. Accumulated during solving; each produces
/// a `TyKind::Error` TyVar that silently absorbs further constraints.
#[derive(Clone, Debug, Hash)]
pub enum InferError {
    /// Types don't match (structural mismatch).
    TypeMismatch {
        expected: TyVar,
        got: TyVar,
        span: Span,
        /// Why `expected` was expected; set by the solver from the
        /// constraint that failed (`with_reason`).
        reason: Reason,
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
    ///
    /// `receiver` is `None` for a receiver-less ambiguity (an overloaded
    /// module-level function call, `f(…)`, which has no receiver type) and
    /// `Some(tv)` for a genuine member ambiguity on a value of that type.
    /// Without the distinction, the free-function case rendered the synthetic
    /// result/placeholder TyVar as `Error` ("Error.f ambiguous", #210).
    AmbiguousMember {
        receiver: Option<TyVar>,
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

    /// The concrete type behind a plain `some P` return is move-only.
    /// Callers treat `some P` as duplicable, so a NotCopyable underlier
    /// would be bit-copied unsoundly; the annotation needs an explicit
    /// `and not Copyable` negative bound.
    OpaqueUnderlierNotCopyable { concrete: TyVar, span: Span },

    /// A `mutating` closure was passed where a non-mutating (`Borrow`/
    /// `Consuming`) closure parameter is expected — the callee never lends a
    /// mutable place, so the closure's write access can't be honored (#106).
    ConventionMismatch { span: Span },

    /// E624 — a closure value's kind does not pass where the expected kind is
    /// required (the directional passing table in closures.md, "Passing: What
    /// Fits Where"). Only the passing table reports here: the signature-level
    /// kind/convention pairing is E625 (a DeclCheck) and calling a
    /// `mutating`-kind closure non-exclusively is the E203 mutability family.
    KindMismatch {
        expected: kestrel_ast::FnTypeKind,
        actual: kestrel_ast::FnTypeKind,
        span: Span,
    },
}

/// Human wording for a closure kind in a diagnostic — "a normal closure",
/// "a 'mutating' closure". Single source of truth for E624's message and its
/// four mirrors.
pub fn describe_fn_kind(kind: kestrel_ast::FnTypeKind) -> String {
    match kind {
        kestrel_ast::FnTypeKind::Normal => "a normal closure".into(),
        kestrel_ast::FnTypeKind::Escaping => "an 'escaping' closure".into(),
        // `keyword()` is the single source of truth for the spelling.
        other => format!("a '{}' closure", other.keyword().unwrap_or("")),
    }
}

/// Spelling of a visibility keyword in a diagnostic.
fn vis_label(v: &Vis) -> &'static str {
    match v {
        Vis::Public => "public",
        Vis::Internal => "internal",
        Vis::Fileprivate => "fileprivate",
        Vis::Private => "private",
    }
}

/// The "why" note for E624 — names the property of the source kind that the
/// expected slot needs and the source cannot supply. Mirrors the rejected
/// cells of the passing table in docs/design/closures.md.
fn kind_mismatch_note(
    expected: kestrel_ast::FnTypeKind,
    actual: kestrel_ast::FnTypeKind,
) -> String {
    use kestrel_ast::FnTypeKind::*;
    match (actual, expected) {
        // Frame views are not owned environments and cannot leave the frame.
        (Normal | Mutating, Consuming) => {
            "a frame-view closure does not own its captures; a 'consuming' slot needs an owned \
             environment"
                .into()
        },
        (Normal | Mutating, Escaping) => {
            "a frame-view closure is frame-bound and can never flow into an 'escaping' slot".into()
        },
        // Exclusive-call values do not weaken to shared calls.
        (Mutating, Normal) => {
            "a 'mutating' closure's calls are exclusive, so it cannot be used where shared calls \
             are allowed"
                .into()
        },
        // One-shot values fit nothing else.
        (Consuming, _) => {
            "a 'consuming' closure runs exactly once and is uniquely owned; it fits only a \
             'consuming' slot"
                .into()
        },
        // Shared handles are not exclusive.
        (Escaping, Mutating) => {
            "an 'escaping' closure is shared (aliases may exist), so its calls are not exclusive"
                .into()
        },
        _ => "see the closure passing table in the language reference".into(),
    }
}

/// The user-facing rendering of an `InferError`: its code, headline message,
/// primary-label text and notes.
///
/// **This is the only description of an inference error in the compiler.**
/// It used to be two full per-variant `match`es — one in `kestrel-compiler`'s
/// codespan renderer, one in `kestrel-analyze`'s `TypeCheckAnalyzer` — which
/// drifted in wording and in code (the same closure-kind mistake shipped as
/// both `E624` and `E100`) and made every type error render twice. Consumers
/// now wrap `InferError::render`; none of them re-derive wording. Adding an
/// `InferError` variant means adding exactly one arm, here.
#[derive(Clone, Debug)]
pub struct RenderedInferError {
    /// Diagnostic code, from `InferError::code` (`docs/error-codes.md`).
    pub code: &'static str,
    /// Headline message, without the code.
    pub message: String,
    /// Text for the primary label at `span()`. `None` renders a bare underline.
    pub label: Option<String>,
    /// Trailing explanatory notes.
    pub notes: Vec<String>,
    /// Secondary labels: other places that explain the error, e.g. the
    /// annotation that set the expected type ("expected because of this").
    pub secondary: Vec<(Span, String)>,
}

impl InferError {
    /// Render this error for a user. `detail` is the resolved-type description
    /// the solver produced alongside it (`TypedBody::error_details`), already
    /// substituted; several variants use it verbatim as their message.
    pub fn render(&self, detail: &str) -> RenderedInferError {
        // Shorthand: `self.code()` + a message + `detail` as the label text — by far
        // the most common shape.
        let detailed = |message: String| RenderedInferError {
            code: self.code(),
            message,
            label: Some(detail.to_string()),
            notes: Vec::new(),
            secondary: Vec::new(),
        };
        // Shorthand: `self.code()` + a message + a fixed label, ignoring `detail`.
        let labeled = |message: String, label: String| RenderedInferError {
            code: self.code(),
            message,
            label: Some(label),
            notes: Vec::new(),
            secondary: Vec::new(),
        };

        match self {
            Self::TypeMismatch { reason, .. } => {
                let mut r = detailed("type mismatch".into());
                reason.explain(&mut r);
                r
            },

            Self::DoesNotConform { .. } => detailed(
                "type mismatch: does not conform to protocol; does not satisfy constraint".into(),
            ),

            // `detail` already carries the full wording ("no method 'X' on type
            // 'Y'"), so it is the message as well as the label.
            Self::NoMember { .. } => detailed(detail.to_string()),

            Self::AmbiguousMember { receiver, name, .. } => {
                // A receiver-less ambiguity is an overloaded free-function
                // call, not a member access — and its detail must not leak the
                // synthetic `Error` placeholder (#210).
                detailed(if receiver.is_some() {
                    format!("ambiguous member '{name}'")
                } else {
                    format!("ambiguous call to '{name}'")
                })
            },

            Self::MemberNotVisible {
                name, visibility, ..
            } => detailed(format!(
                "member '{name}' is {} and not accessible from this scope",
                vis_label(visibility)
            )),

            Self::MemberIsStatic { name, .. } => labeled(
                format!("'{name}' is a static member and cannot be used on an instance"),
                format!("use the type name to call '{name}'"),
            ),

            Self::NoAssociatedType { name, .. } => detailed(format!("no associated type '{name}'")),

            Self::InfiniteType { .. } => {
                labeled("infinite type".into(), "recursive type detected".into())
            },

            // Propagated from an earlier phase (parse / name resolution), which
            // already reported the real error — carry no label text of its own.
            Self::FromHir { .. } => RenderedInferError {
                code: self.code(),
                message: "error in expression".into(),
                label: None,
                notes: Vec::new(),
                secondary: Vec::new(),
            },

            Self::ImplicitMemberNotFound { name, .. } => {
                detailed(format!("implicit member '.{name}' not found"))
            },

            Self::ArgCountMismatch { expected, got, .. } => detailed(format!(
                "wrong number of arguments: expected {expected}, got {got}"
            )),

            Self::LabelMismatch { .. } => detailed("wrong argument label".into()),

            Self::InstanceMethodAsStatic { name, .. } => {
                detailed(format!("instance method '{name}' cannot be called on a type"))
            },

            Self::TypeParamAsValue { .. } => labeled(
                "type parameter cannot be used as a value".into(),
                "not a value".into(),
            ),

            Self::TypeArgCountMismatch { expected, got, .. } => detailed(if got < expected {
                format!("too few type arguments: expected {expected}, got {got}")
            } else {
                format!("too many type arguments: expected {expected}, got {got}")
            }),

            Self::NoMatchingOverload { name, .. } => {
                detailed(format!("no matching overload for '{name}'"))
            },

            Self::MemberwiseInitArity {
                struct_name,
                expected,
                got,
                ..
            } => labeled(
                format!(
                    "struct '{struct_name}' has {expected} field(s), but {got} argument(s) were provided"
                ),
                format!("expected {expected} argument(s)"),
            ),

            Self::MemberwiseInitLabel {
                struct_name,
                expected,
                got,
                ..
            } => {
                let got_desc = got
                    .as_deref()
                    .map(|s| format!("'{s}'"))
                    .unwrap_or_else(|| "unlabeled".into());
                labeled(
                    format!(
                        "argument for struct '{struct_name}' has {got_desc} label, but expected '{expected}'"
                    ),
                    format!("expected label '{expected}'"),
                )
            },

            Self::ItWrongArity { expected, .. } => labeled(
                "implicit 'it' parameter requires single-parameter context".into(),
                format!("expected {expected} parameter(s)"),
            ),

            Self::LiteralNotAccepted { .. } => {
                detailed("type mismatch: does not conform to protocol".into())
            },

            Self::UnresolvedTypeParam { .. } => RenderedInferError {
                code: self.code(),
                message: "cannot infer type parameter".into(),
                label: Some(detail.to_string()),
                notes: vec![
                    "no argument or context constrains this type parameter; \
                     annotate it explicitly at the call (e.g. `f[_, Int64](...)`) \
                     or at the binding (e.g. `let x: T = f(...)`)"
                        .into(),
                ],
                secondary: Vec::new(),
            },

            Self::CannotInferType { .. } => labeled(
                "could not infer type".into(),
                "add a type annotation to resolve this".into(),
            ),

            Self::TupleIndexOnNonTuple { index, .. } => labeled(
                format!("cannot index into non-tuple type: {detail}"),
                format!("'.{index}' requires a tuple receiver"),
            ),

            Self::TupleIndexOutOfBounds { arity, index, .. } => labeled(
                format!("tuple index {index} out of bounds for {arity}-element tuple"),
                format!("valid indices are 0..{}", arity.saturating_sub(1)),
            ),

            Self::MemberAccessOnPrimitive { name, .. } => labeled(
                format!("cannot access member on type: {detail}"),
                format!("'{name}' not available"),
            ),

            Self::MethodNotCalled { method, .. } => RenderedInferError {
                code: self.code(),
                message: detail.to_string(),
                label: Some("add () to call this method".into()),
                notes: vec![format!(
                    "primitive methods cannot be used as first-class values; use '.{method}()' instead"
                )],
                secondary: Vec::new(),
            },

            Self::CircularOpaqueReturn { .. } => RenderedInferError {
                code: self.code(),
                message: "circular opaque return type".into(),
                label: Some("concrete type cannot be determined".into()),
                notes: vec![
                    "mutually recursive functions with 'some' return types must have at least one non-opaque base case".into(),
                ],
                secondary: Vec::new(),
            },

            Self::OpaqueUnderlierNotCopyable { .. } => RenderedInferError {
                code: self.code(),
                message: "opaque return type hides a non-Copyable type".into(),
                label: Some(detail.to_string()),
                notes: vec![
                    "a plain 'some P' promises callers a Copyable value; write 'some P and not Copyable' to allow a move-only concrete type".into(),
                ],
                secondary: Vec::new(),
            },

            Self::ConventionMismatch { .. } => labeled(
                "convention mismatch: cannot pass a mutating closure where a non-mutating parameter is expected".into(),
                "mutating closure not allowed here".into(),
            ),

            // E624: the closure passing table (closures.md "Passing: What Fits
            // Where"). ONLY the table reports here — the signature-level
            // kind/convention pairing is E625 (a DeclCheck) and a non-exclusive
            // `mutating` call is the E203 mutability family.
            Self::KindMismatch {
                expected, actual, ..
            } => RenderedInferError {
                code: self.code(),
                message: format!(
                    "closure kind mismatch: expected {}, found {}",
                    describe_fn_kind(*expected),
                    describe_fn_kind(*actual)
                ),
                label: Some(format!("this is {}", describe_fn_kind(*actual))),
                notes: vec![kind_mismatch_note(*expected, *actual)],
                secondary: Vec::new(),
            },

            Self::RefFunctionAsValue { .. } => RenderedInferError {
                code: self.code(),
                message: "a reference-returning function cannot be used as a value".into(),
                label: Some(
                    "call it instead — `-> &T` is a return convention, not part of a \
                     function type"
                        .into(),
                ),
                notes: vec![
                    "capturing or storing it would erase the ret_borrow calling convention".into(),
                ],
                secondary: Vec::new(),
            },

            Self::RefInTypeArgument { .. } => RenderedInferError {
                code: self.code(),
                message: "a reference cannot be a generic type argument".into(),
                label: Some(
                    "this would store the reference; references are second-class".into(),
                ),
                notes: vec![
                    "bind the value first (`let x = ...;`) to store an owned copy".into(),
                ],
                secondary: Vec::new(),
            },
        }
    }

    /// The source span where this error occurred.
    /// Attach the cause of the expectation to a type mismatch; other errors
    /// are returned unchanged. A reason already present is kept: it is the
    /// more specific one.
    pub fn with_reason(mut self, reason: &Reason) -> Self {
        if let Self::TypeMismatch { reason: r, .. } = &mut self
            && *r == Reason::Unspecified
        {
            *r = reason.clone();
        }
        self
    }

    /// The diagnostic code for this error. One exhaustive match — no
    /// wildcard — so a new variant must choose its code here; `render` reads
    /// it from nowhere else. Codes and their meanings are listed in
    /// `docs/error-codes.md` (E102–E118). E100 is left only on `FromHir`,
    /// which is never shown (HIR lowering already reported the real error).
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoMember { .. }
            | Self::MemberAccessOnPrimitive { .. }
            | Self::NoAssociatedType { .. }
            | Self::TupleIndexOnNonTuple { .. }
            | Self::TupleIndexOutOfBounds { .. } => "E102",
            Self::AmbiguousMember { .. } => "E103",
            Self::ImplicitMemberNotFound { .. } => "E104",
            Self::CannotInferType { .. } | Self::UnresolvedTypeParam { .. } => "E105",
            Self::DoesNotConform { .. } => "E107",
            Self::ItWrongArity { .. } => "E108",
            Self::TypeMismatch { .. } | Self::ConventionMismatch { .. } => "E109",
            Self::ArgCountMismatch { .. }
            | Self::LabelMismatch { .. }
            | Self::MemberwiseInitArity { .. }
            | Self::MemberwiseInitLabel { .. }
            | Self::NoMatchingOverload { .. }
            | Self::TypeArgCountMismatch { .. } => "E113",
            Self::LiteralNotAccepted { .. } => "E114",
            Self::MemberNotVisible { .. } => "E115",
            Self::MemberIsStatic { .. }
            | Self::InstanceMethodAsStatic { .. }
            | Self::TypeParamAsValue { .. }
            | Self::MethodNotCalled { .. } => "E116",
            Self::InfiniteType { .. } => "E117",
            Self::CircularOpaqueReturn { .. } | Self::OpaqueUnderlierNotCopyable { .. } => "E118",
            Self::KindMismatch { .. } => "E624",
            Self::RefFunctionAsValue { .. } => "E491",
            Self::RefInTypeArgument { .. } => "E492",
            Self::FromHir { .. } => "E100",
        }
    }

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
            | Self::OpaqueUnderlierNotCopyable { span, .. }
            | Self::RefFunctionAsValue { span }
            | Self::RefInTypeArgument { span }
            | Self::ConventionMismatch { span }
            | Self::KindMismatch { span, .. } => span,
        }
    }
}

impl Reason {
    /// Add this cause of an expectation to a rendered mismatch: a secondary
    /// label at the cause, or a note when there is no real span to point at.
    fn explain(&self, r: &mut RenderedInferError) {
        let (span, text) = match self {
            Reason::Unspecified => return,
            Reason::Annotation(s) => (s, "expected because of this annotation"),
            Reason::Return(s) => (s, "expected because of this return type"),
            Reason::Param { decl: Some(s), .. } => (s, "expected because of this parameter"),
            Reason::Param { index, decl: None } => {
                r.notes.push(format!(
                    "expected because of the type of parameter {}",
                    index + 1
                ));
                return;
            },
            Reason::FirstArm(s) => (s, "expected because the first branch has this type"),
            Reason::Element(s) => (s, "expected because the first element has this type"),
            Reason::Assign(s) => (s, "expected because of the type of this target"),
        };
        if !span.is_synthetic() {
            r.secondary.push((span.clone(), text.into()));
        }
    }
}
