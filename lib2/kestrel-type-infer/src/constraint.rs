//! Constraint definitions for type inference.
//!
//! Seven constraint variants cover the entire type system. Each is emitted
//! during constraint generation and consumed by the fixpoint solver.

use kestrel_ast_builder::AstParam;
use kestrel_hecs::Entity;
use kestrel_hir::body::HirExprId;
use kestrel_hir::ty::HirTy;
use kestrel_span2::Span;

use crate::ty::TyVar;

/// A type constraint emitted during constraint generation.
#[derive(Clone, Debug)]
pub enum Constraint {
    /// `a = b` — structural type equality.
    /// Used where types must be identical: if/match branches, array elements.
    Equal {
        a: TyVar,
        b: TyVar,
        span: Span,
    },

    /// `from → to` — value flows from source to target.
    /// Tries Equal first; on failure, falls back to promotion (FromValue).
    /// Used at value boundaries: let bindings, arguments, return, assignment.
    Coerce {
        from: TyVar,
        to: TyVar,
        expr: HirExprId,
        span: Span,
    },

    /// `ty : Protocol` — protocol conformance.
    /// Deferred until ty is concrete.
    Conforms {
        ty: TyVar,
        protocol: Entity,
        span: Span,
    },

    /// `Container.Name → result` — associated type projection.
    /// Deferred until container is concrete.
    Associated {
        container: TyVar,
        name: String,
        result: TyVar,
        span: Span,
    },

    /// `receiver.name(args) → result` — member resolution.
    /// Covers methods, fields, computed properties, subscripts, inits.
    /// Deferred until receiver type is concrete.
    Member {
        receiver: TyVar,
        name: String,
        args: Vec<CallArg>,
        result: TyVar,
        expr: HirExprId,
        /// True when this came from a call site (MethodCall/ProtocolCall),
        /// false for plain field/property access. Needed to distinguish
        /// `self.f()` (call zero-arg function field) from `self.f` (read field).
        is_call: bool,
        /// True when this is a static context call (e.g., `T.method()` where
        /// T is a type parameter). Only static and init members are valid.
        is_static_context: bool,
        span: Span,
    },

    /// `callee(args) → result` — function or subscript call.
    /// Deferred until callee type is concrete.
    /// If callee is a Function type, unifies params/return directly.
    /// If callee is a Named type, resolves subscript via member system.
    Call {
        callee: TyVar,
        args: Vec<CallArg>,
        result: TyVar,
        expr: HirExprId,
        span: Span,
    },

    /// Overloaded call: one of `candidates` is the correct target.
    /// Solver disambiguates by label/arity, then type compatibility.
    OverloadedCall {
        candidates: Vec<Entity>,
        /// Explicit type args from the call site (e.g., `foo[Int](x)`)
        type_args: Vec<HirTy>,
        args: Vec<CallArg>,
        result: TyVar,
        expr: HirExprId,
        span: Span,
    },

    /// `.name(args) → result` — implicit enum member.
    /// Resolved against the expected type. Deferred until expected type is concrete.
    Implicit {
        expected: TyVar,
        name: String,
        args: Vec<CallArg>,
        result: TyVar,
        expr: HirExprId,
        span: Span,
    },

    /// `.Name(bindings)` in pattern position — implicit variant destructuring.
    /// Deferred until scrutinee type is concrete, then looks up the case by name
    /// and equates each binding TyVar with the corresponding payload type.
    ImplicitPat {
        scrutinee: TyVar,
        name: String,
        /// TyVars for each sub-pattern binding (one per payload field).
        arg_tys: Vec<TyVar>,
        span: Span,
    },

    /// `(prefix.., suffix..)` — tuple pattern with rest.
    /// Deferred until scrutinee resolves to a concrete tuple type, then equates
    /// prefix TyVars against the first N elements and suffix TyVars against the last M.
    TupleRestPat {
        scrutinee: TyVar,
        prefix_tys: Vec<TyVar>,
        suffix_tys: Vec<TyVar>,
        span: Span,
    },
}

/// Argument in a call: optional label + type variable.
#[derive(Clone, Debug)]
pub struct CallArg {
    pub label: Option<String>,
    pub ty: TyVar,
}

/// Check if call arg labels match a callable's param labels.
/// Compares label and arity: arg count must equal param count,
/// and each arg label must match the corresponding param label.
pub fn labels_match(params: &[AstParam], arg_labels: &[Option<&str>]) -> bool {
    if params.len() != arg_labels.len() {
        return false;
    }
    params
        .iter()
        .zip(arg_labels.iter())
        .all(|(param, arg_label)| param.label.as_deref() == *arg_label)
}
