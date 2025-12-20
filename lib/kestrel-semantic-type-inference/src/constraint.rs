//! Type inference constraints.
//!
//! Constraints represent relationships between types that must hold.
//! The solver processes these constraints to find a consistent type assignment.

use kestrel_semantic_tree::expr::ExprId;
use kestrel_semantic_tree::ty::TyId;
use kestrel_span::Span;
use semantic_tree::symbol::SymbolId;

/// Reference to a protocol for conformance constraints.
///
/// Stores the protocol's symbol ID and any type argument substitutions.
#[derive(Debug, Clone)]
pub struct ProtocolRef {
    /// The protocol symbol ID
    pub symbol_id: SymbolId,
    /// Span where the conformance requirement originates
    pub span: Span,
}

impl ProtocolRef {
    /// Create a new protocol reference.
    pub fn new(symbol_id: SymbolId, span: Span) -> Self {
        Self { symbol_id, span }
    }
}

/// A type inference constraint.
///
/// Constraints are collected during expression resolution and then solved
/// by the inference context using unification and fixpoint iteration.
#[derive(Debug, Clone)]
pub enum Constraint {
    /// Two types must be equal: τ₁ = τ₂
    ///
    /// This is the fundamental unification constraint. When solved, it
    /// produces substitutions that make both types identical.
    Equals {
        /// The first type
        a: TyId,
        /// The second type
        b: TyId,
        /// Span for error reporting (where the constraint originates)
        span: Span,
    },

    /// A type must conform to a protocol: τ : Protocol
    ///
    /// This constraint verifies that a type implements all requirements
    /// of a protocol. It's used for generic bounds and protocol contexts.
    Conforms {
        /// The type that must conform
        ty: TyId,
        /// The protocol it must conform to
        protocol: ProtocolRef,
    },

    /// Associated type normalization: Container.AssocType => τ
    ///
    /// This constraint resolves an associated type projection to a concrete type.
    /// For example, `Iterator.Item` where `Iterator` is `ArrayIterator[Int]` resolves to `Int`.
    Normalizes {
        /// The container type (e.g., the type with the associated type)
        base: TyId,
        /// The name of the associated type
        assoc_name: String,
        /// The result type that the projection resolves to
        result: TyId,
        /// Span for error reporting
        span: Span,
    },

    /// Member access constraint: receiver.member has type τ
    ///
    /// This constraint resolves type-directed member lookups. It's needed when
    /// the receiver type isn't yet known at constraint generation time.
    MemberAccess {
        /// The receiver type being accessed
        receiver: TyId,
        /// The member name
        member: String,
        /// Whether this is a static member access (Type.member vs instance.member)
        is_static: bool,
        /// The result type of the member access
        result: TyId,
        /// The expression ID for tracking the value resolution
        expr_id: ExprId,
        /// Span for error reporting
        span: Span,
    },

    /// Implicit member access for enum shorthand: .Case or .Case(args)
    ///
    /// Resolved when the expression's expected type becomes known through
    /// unification with context (e.g., parameter type, return type, binding type).
    ImplicitMember {
        /// The expression's type (starts as Infer, unified with expected type)
        expr_ty: TyId,
        /// The member/case name
        member_name: String,
        /// Argument type IDs if present (for associated values)
        /// Each entry is (optional label, type_id)
        argument_tys: Vec<(Option<String>, TyId)>,
        /// Expression ID for value resolution recording
        expr_id: ExprId,
        /// Span for error reporting
        span: Span,
    },
}

impl Constraint {
    /// Create an equality constraint.
    pub fn equals(a: TyId, b: TyId, span: Span) -> Self {
        Constraint::Equals { a, b, span }
    }

    /// Create a conformance constraint.
    pub fn conforms(ty: TyId, protocol: ProtocolRef) -> Self {
        Constraint::Conforms { ty, protocol }
    }

    /// Create a normalization constraint.
    pub fn normalizes(base: TyId, assoc_name: String, result: TyId, span: Span) -> Self {
        Constraint::Normalizes {
            base,
            assoc_name,
            result,
            span,
        }
    }

    /// Create a member access constraint.
    pub fn member_access(
        receiver: TyId,
        member: String,
        is_static: bool,
        result: TyId,
        expr_id: ExprId,
        span: Span,
    ) -> Self {
        Constraint::MemberAccess {
            receiver,
            member,
            is_static,
            result,
            expr_id,
            span,
        }
    }

    /// Get the span associated with this constraint (for error reporting).
    pub fn span(&self) -> &Span {
        match self {
            Constraint::Equals { span, .. } => span,
            Constraint::Conforms { protocol, .. } => &protocol.span,
            Constraint::Normalizes { span, .. } => span,
            Constraint::MemberAccess { span, .. } => span,
            Constraint::ImplicitMember { span, .. } => span,
        }
    }

    /// Create an implicit member access constraint.
    pub fn implicit_member(
        expr_ty: TyId,
        member_name: String,
        argument_tys: Vec<(Option<String>, TyId)>,
        expr_id: ExprId,
        span: Span,
    ) -> Self {
        Constraint::ImplicitMember {
            expr_ty,
            member_name,
            argument_tys,
            expr_id,
            span,
        }
    }
}
