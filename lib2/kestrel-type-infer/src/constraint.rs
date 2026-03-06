//! Constraint definitions for type inference.
//!
//! Seven constraint variants cover the entire type system. Each is emitted
//! during constraint generation and consumed by the fixpoint solver.

use kestrel_hecs::Entity;
use kestrel_hir::body::HirExprId;
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
}

/// Argument in a call: optional label + type variable.
#[derive(Clone, Debug)]
pub struct CallArg {
    pub label: Option<String>,
    pub ty: TyVar,
}
