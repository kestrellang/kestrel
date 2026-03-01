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
    ///
    /// For method calls (DeferredMethodCall), this also includes argument types
    /// so that parameter constraints can be created when the method is resolved.
    MemberAccess {
        /// The receiver type being accessed
        receiver: TyId,
        /// The member name
        member: String,
        /// Whether this is a static member access (Type.member vs instance.member)
        is_static: bool,
        /// Argument type IDs for method calls (empty for field access)
        /// Used to create constraints between argument types and parameter types
        arguments: Vec<TyId>,
        /// Argument labels for method calls (empty for field access).
        /// Used for overload resolution when multiple methods share the same name.
        /// Each entry is None for unlabeled arguments, Some(label) for labeled ones.
        labels: Vec<Option<String>>,
        /// Whether this is a non-call property access (DeferredMemberAccess),
        /// as opposed to a method call with zero arguments.
        is_property_access: bool,
        /// The result type of the member access
        result: TyId,
        /// The expression ID for tracking the value resolution
        expr_id: ExprId,
        /// Substitutions from the call site (includes inference variables for method type params)
        substitutions: kestrel_semantic_tree::ty::Substitutions,
        /// Explicit type arguments from the call site (e.g., `x.map[Int64](1)`).
        /// Converted to substitutions by the solver after resolving the method.
        explicit_type_args: Option<Vec<kestrel_semantic_tree::ty::Ty>>,
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

    /// Enum pattern binding constraint: binds pattern types to enum case parameter types.
    ///
    /// When matching `.Some(value)`, the type of `value` must match the `Some` case's
    /// parameter type. This constraint defers the binding until the enum type is known.
    EnumPatternBinding {
        /// The enum type (pattern's type, which equals the scrutinee type)
        enum_ty: TyId,
        /// The case name being matched (e.g., "Some")
        case_name: String,
        /// Binding types: each entry is (optional label, binding pattern's TyId)
        binding_tys: Vec<(Option<String>, TyId)>,
        /// Span for error reporting
        span: Span,
    },

    /// Struct pattern binding constraint: binds pattern types to struct field types.
    ///
    /// When matching `Point { x, y }`, the types of `x` and `y` bindings must match
    /// the `Point` struct's field types. This constraint defers until the struct type is known.
    StructPatternBinding {
        /// The struct type (pattern's type, which equals the scrutinee type)
        struct_ty: TyId,
        /// The struct name as written in the pattern
        struct_name: String,
        /// Field bindings: each entry is (field_name, binding pattern's TyId)
        field_bindings: Vec<(String, TyId)>,
        /// Whether the pattern has a rest pattern (`..`) to ignore extra fields
        has_rest: bool,
        /// Span for error reporting
        span: Span,
    },

    /// A value may be promoted to a target type via `FromValue`.
    ///
    /// First tries unification. If that fails, checks if the target type
    /// conforms to `FromValue[source]` and records a promotion if so.
    /// Used for assignments, returns, and function arguments.
    Promotable {
        /// The source expression's type (the value being assigned)
        from_ty: TyId,
        /// The target type to assign to (e.g., `Optional[T]`)
        to_ty: TyId,
        /// The expression that may need wrapping
        expr_id: ExprId,
        /// Span for error reporting
        span: Span,
    },

    /// Tuple index access constraint: tuple.index has type τ
    ///
    /// This constraint resolves tuple indexing when the tuple type isn't yet known
    /// at constraint generation time (e.g., type parameters with tuple constraints).
    TupleIndexAccess {
        /// The tuple type being indexed
        tuple: TyId,
        /// The index being accessed
        index: usize,
        /// The result type of the index access
        result: TyId,
        /// Span for error reporting
        span: Span,
    },

    /// Function call constraint: resolves direct function calls and overloaded calls.
    ///
    /// This constraint handles calls to free functions and overloaded functions,
    /// deferring resolution until argument types are known for type-directed
    /// overload selection.
    FunctionCall {
        /// Candidate function symbol IDs
        candidates: Vec<SymbolId>,
        /// Argument type IDs
        arguments: Vec<TyId>,
        /// Argument expression IDs (for recording promotions)
        argument_expr_ids: Vec<ExprId>,
        /// Argument labels for overload resolution
        labels: Vec<Option<String>>,
        /// Explicit type arguments from the call site
        explicit_type_args: Option<Vec<kestrel_semantic_tree::ty::Ty>>,
        /// The result type of the function call
        result: TyId,
        /// The expression ID for tracking the value resolution
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
        arguments: Vec<TyId>,
        labels: Vec<Option<String>>,
        result: TyId,
        expr_id: ExprId,
        substitutions: kestrel_semantic_tree::ty::Substitutions,
        explicit_type_args: Option<Vec<kestrel_semantic_tree::ty::Ty>>,
        span: Span,
    ) -> Self {
        Constraint::MemberAccess {
            receiver,
            member,
            is_static,
            arguments,
            labels,
            is_property_access: false,
            result,
            expr_id,
            substitutions,
            explicit_type_args,
            span,
        }
    }

    pub fn property_access(
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
            arguments: vec![],
            labels: vec![],
            is_property_access: true,
            result,
            expr_id,
            substitutions: kestrel_semantic_tree::ty::Substitutions::new(),
            explicit_type_args: None,
            span,
        }
    }

    /// Create a tuple index access constraint.
    pub fn tuple_index_access(tuple: TyId, index: usize, result: TyId, span: Span) -> Self {
        Constraint::TupleIndexAccess {
            tuple,
            index,
            result,
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
            Constraint::EnumPatternBinding { span, .. } => span,
            Constraint::StructPatternBinding { span, .. } => span,
            Constraint::Promotable { span, .. } => span,
            Constraint::TupleIndexAccess { span, .. } => span,
            Constraint::FunctionCall { span, .. } => span,
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

    /// Create an enum pattern binding constraint.
    pub fn enum_pattern_binding(
        enum_ty: TyId,
        case_name: String,
        binding_tys: Vec<(Option<String>, TyId)>,
        span: Span,
    ) -> Self {
        Constraint::EnumPatternBinding {
            enum_ty,
            case_name,
            binding_tys,
            span,
        }
    }

    /// Create a struct pattern binding constraint.
    pub fn struct_pattern_binding(
        struct_ty: TyId,
        struct_name: String,
        field_bindings: Vec<(String, TyId)>,
        has_rest: bool,
        span: Span,
    ) -> Self {
        Constraint::StructPatternBinding {
            struct_ty,
            struct_name,
            field_bindings,
            has_rest,
            span,
        }
    }

    /// Create a promotable constraint.
    pub fn promotable(from_ty: TyId, to_ty: TyId, expr_id: ExprId, span: Span) -> Self {
        Constraint::Promotable {
            from_ty,
            to_ty,
            expr_id,
            span,
        }
    }
}
