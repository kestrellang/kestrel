//! Inference context for collecting and solving constraints.
//!
//! The [`InferenceContext`] is the main entry point for type inference.
//! It collects constraints during expression resolution and then solves
//! them to produce a [`Solution`].

use std::collections::{HashMap, HashSet};

use kestrel_semantic_tree::expr::ExprId;
use kestrel_semantic_tree::ty::{ParamInfo, Ty, TyId, TyKind};
use kestrel_span::Span;
use semantic_tree::symbol::Symbol;

use crate::constraint::{Constraint, ProtocolRef};
use crate::error::InferenceError;
use crate::oracle::TypeOracle;
use crate::solution::{MemberKind, PromotionInfo, Solution, ValueResolution};

/// Metadata about a closure expression for error reporting.
#[derive(Debug, Clone)]
pub struct ClosureMetadata {
    /// The expression ID of the closure
    pub expr_id: ExprId,
    /// Number of parameters in the closure
    pub param_count: usize,
    /// Whether the closure uses the implicit `it` parameter
    pub uses_it: bool,
    /// Whether the closure has explicit parameters (vs implicit)
    pub has_explicit_params: bool,
    /// The span of the closure expression
    pub span: Span,
    /// The type ID of the closure (function type)
    pub ty_id: TyId,
}

/// Context for collecting and solving type inference constraints.
///
/// # Usage
///
/// 1. Create a context with a [`TypeOracle`] implementation
/// 2. Add constraints using `equate`, `conforms`, `normalizes`, `member_access`
/// 3. Call `solve()` to get a [`Solution`]
///
/// # Example
///
/// ```ignore
/// let mut ctx = InferenceContext::new(&oracle);
///
/// // During expression resolution...
/// ctx.equate(expected_ty.id(), actual_ty.id(), span);
/// ctx.member_access(receiver.id(), "field", false, result.id(), expr.id(), span);
///
/// // After collecting all constraints...
/// let solution = ctx.solve()?;
/// ```
pub struct InferenceContext<'a> {
    /// The type oracle for querying type information
    oracle: &'a dyn TypeOracle,
    /// Pending constraints to solve
    constraints: Vec<Constraint>,
    /// Current type substitutions (TyId -> resolved Ty)
    substitutions: HashMap<TyId, Ty>,
    /// Current value resolutions (ExprId -> ValueResolution)
    values: HashMap<ExprId, ValueResolution>,
    /// Promotions for expressions that need FromValue.from() wrapping
    promotions: HashMap<ExprId, PromotionInfo>,
    /// Map from TyId to its original Ty (for looking up spans and kinds)
    type_registry: HashMap<TyId, Ty>,
    /// Accumulated errors during solving
    errors: Vec<InferenceError>,
    /// Closure metadata for error reporting (TyId -> ClosureMetadata)
    closure_metadata: HashMap<TyId, ClosureMetadata>,
    /// The expected return type for the current function/method body
    return_type: Option<Ty>,
    /// TyIds that have ExpressibleBy* constraints (i.e., literals)
    /// Used by Promotable solver to decide whether to defer
    literal_ty_ids: HashSet<TyId>,
    /// Member kind classifications for deferred member access
    member_kinds: HashMap<ExprId, MemberKind>,
}

impl<'a> InferenceContext<'a> {
    /// Create a new inference context with the given oracle.
    pub fn new(oracle: &'a dyn TypeOracle) -> Self {
        Self {
            oracle,
            constraints: Vec::new(),
            substitutions: HashMap::new(),
            values: HashMap::new(),
            promotions: HashMap::new(),
            type_registry: HashMap::new(),
            errors: Vec::new(),
            closure_metadata: HashMap::new(),
            return_type: None,
            literal_ty_ids: HashSet::new(),
            member_kinds: HashMap::new(),
        }
    }

    /// Register a type for tracking during inference.
    ///
    /// This should be called for any type that might be unified or
    /// referenced in constraints. It allows looking up the original
    /// type from its ID.
    ///
    /// This recursively registers nested types (e.g., element types of arrays,
    /// tuple elements, function params/returns) so they can be found during
    /// constraint solving.
    pub fn register_type(&mut self, ty: &Ty) {
        // Avoid duplicate registration
        if self.type_registry.contains_key(&ty.id()) {
            return;
        }

        self.type_registry.insert(ty.id(), ty.clone());

        // Recursively register nested types
        match ty.kind() {
            TyKind::Pointer(elem_ty) => {
                self.register_type(elem_ty);
            },
            // Note: Array[T] struct types have their substitutions registered via the Struct case below
            TyKind::Tuple(elem_tys) => {
                for elem_ty in elem_tys {
                    self.register_type(elem_ty);
                }
            },
            TyKind::Function {
                params,
                return_type,
            } => {
                for param_ty in params {
                    self.register_type(param_ty);
                }
                self.register_type(return_type);
            },
            TyKind::Struct { substitutions, .. }
            | TyKind::Enum { substitutions, .. }
            | TyKind::Protocol { substitutions, .. }
            | TyKind::TypeAlias { substitutions, .. } => {
                for (_, sub_ty) in substitutions.iter() {
                    self.register_type(sub_ty);
                }
            },
            TyKind::AssociatedType { container, .. } => {
                if let Some(container_ty) = container {
                    self.register_type(container_ty);
                }
            },
            TyKind::UnresolvedFunction {
                param_info,
                return_type,
            } => {
                self.register_type(return_type);
                match param_info {
                    ParamInfo::ImplicitIt { it_type } => {
                        self.register_type(it_type);
                    },
                    ParamInfo::Explicit { param_types } => {
                        for pt in param_types {
                            self.register_type(pt);
                        }
                    },
                    ParamInfo::Unconstrained => {},
                }
            },
            // Leaf types - no nested types to register
            TyKind::Int(_)
            | TyKind::Float(_)
            | TyKind::Bool
            | TyKind::String
            | TyKind::Unit
            | TyKind::Never
            | TyKind::Infer
            | TyKind::Error
            | TyKind::SelfType
            | TyKind::TypeParameter(_)
            | TyKind::UnresolvedPath { .. } => {},
        }
    }

    /// Get a registered type by ID.
    pub fn get_type(&self, id: TyId) -> Option<&Ty> {
        // First check substitutions, then registry
        self.substitutions
            .get(&id)
            .or_else(|| self.type_registry.get(&id))
    }

    // === Constraint Addition Methods ===

    /// Add an equality constraint: the two types must be equal.
    ///
    /// This is used when two types must match, e.g., function argument
    /// type must match parameter type.
    pub fn equate(&mut self, a: TyId, b: TyId, span: Span) {
        self.constraints.push(Constraint::equals(a, b, span));
    }

    /// Add a conformance constraint: the type must conform to the protocol.
    ///
    /// This is used for generic bounds, protocol contexts, etc.
    pub fn conforms(&mut self, ty: TyId, protocol: ProtocolRef) {
        self.constraints.push(Constraint::conforms(ty, protocol));
    }

    /// Add a normalization constraint: the associated type must resolve to result.
    ///
    /// This is used for associated type projections like `T.Item`.
    pub fn normalizes(&mut self, base: TyId, assoc_name: String, result: TyId, span: Span) {
        self.constraints
            .push(Constraint::normalizes(base, assoc_name, result, span));
    }

    /// Add a member access constraint: accessing member on receiver yields result.
    ///
    /// This is used when resolving `receiver.member` where the receiver type
    /// is not yet known.
    ///
    /// For method calls, `arguments` contains the type IDs of the call arguments.
    /// When the method is resolved, constraints will be created to equate
    /// argument types with parameter types (enabling proper type inference for literals).
    ///
    /// `substitutions` contains the call-site substitutions (including inference variables
    /// for method type parameters created during binding).
    pub fn member_access(
        &mut self,
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
    ) {
        self.constraints.push(Constraint::member_access(
            receiver,
            member,
            is_static,
            arguments,
            labels,
            result,
            expr_id,
            substitutions,
            explicit_type_args,
            span,
        ));
    }

    /// Add a property access constraint (non-call member access).
    pub fn property_access(
        &mut self,
        receiver: TyId,
        member: String,
        is_static: bool,
        result: TyId,
        expr_id: ExprId,
        span: Span,
    ) {
        self.constraints.push(Constraint::property_access(
            receiver, member, is_static, result, expr_id, span,
        ));
    }

    /// Add an implicit member constraint: resolve `.Member` or `.Member(args)` based on expected type.
    ///
    /// This is used for enum shorthand syntax where the enum type is inferred from context.
    pub fn implicit_member(
        &mut self,
        expr_ty: TyId,
        member_name: String,
        argument_tys: Vec<(Option<String>, TyId)>,
        expr_id: ExprId,
        span: Span,
    ) {
        self.constraints.push(Constraint::implicit_member(
            expr_ty,
            member_name,
            argument_tys,
            expr_id,
            span,
        ));
    }

    /// Add an enum pattern binding constraint.
    ///
    /// This is used when matching enum patterns like `.Some(value)` to connect
    /// the binding's type to the enum case's parameter type.
    pub fn enum_pattern_binding(
        &mut self,
        enum_ty: TyId,
        case_name: String,
        binding_tys: Vec<(Option<String>, TyId)>,
        span: Span,
    ) {
        self.constraints.push(Constraint::enum_pattern_binding(
            enum_ty,
            case_name,
            binding_tys,
            span,
        ));
    }

    /// Add a struct pattern binding constraint.
    ///
    /// This is used when matching struct patterns like `Point { x, y }` to connect
    /// the binding's type to the struct's field types.
    pub fn struct_pattern_binding(
        &mut self,
        struct_ty: TyId,
        struct_name: String,
        field_bindings: Vec<(String, TyId)>,
        has_rest: bool,
        span: Span,
    ) {
        self.constraints.push(Constraint::struct_pattern_binding(
            struct_ty,
            struct_name,
            field_bindings,
            has_rest,
            span,
        ));
    }

    /// Add a tuple index access constraint.
    ///
    /// This is used when the tuple type isn't yet known at constraint generation time
    /// (e.g., type parameters with tuple constraints). The solver will resolve the
    /// element type once the tuple type becomes known.
    pub fn tuple_index_access(&mut self, tuple: TyId, index: usize, result: TyId, span: Span) {
        self.constraints
            .push(Constraint::tuple_index_access(tuple, index, result, span));
    }

    /// Add a promotable constraint: the value may be promoted to the target type.
    ///
    /// This first tries unification. If that fails, it checks if the target type
    /// conforms to `FromValue[source]` and records a promotion if so.
    ///
    /// Used for assignments, returns, and function arguments to enable implicit
    /// wrapping of values in Optional or Result types.
    pub fn promotable(&mut self, from_ty: TyId, to_ty: TyId, expr_id: ExprId, span: Span) {
        self.constraints
            .push(Constraint::promotable(from_ty, to_ty, expr_id, span));
    }

    /// Register metadata for a closure expression.
    ///
    /// This should be called during constraint generation for closures
    /// to enable better error messages during type inference.
    pub fn register_closure_metadata(&mut self, metadata: ClosureMetadata) {
        self.closure_metadata.insert(metadata.ty_id, metadata);
    }

    /// Set the expected return type for the current function body.
    ///
    /// This is used to check that `return` statements have the correct type.
    pub fn set_return_type(&mut self, ty: Option<Ty>) {
        self.return_type = ty;
    }

    /// Get the expected return type, if set.
    pub fn return_type(&self) -> Option<&Ty> {
        self.return_type.as_ref()
    }

    // === Solving ===

    /// Solve all collected constraints and return a solution.
    ///
    /// This consumes the context and returns a [`Solution`] containing
    /// all resolved types, values, and any errors encountered during
    /// inference. Errors are accumulated rather than failing fast.
    pub fn solve(self) -> Solution {
        crate::solver::solve(self)
    }

    // === Internal accessors for solver ===

    pub(crate) fn oracle(&self) -> &dyn TypeOracle {
        self.oracle
    }

    pub(crate) fn take_constraints(&mut self) -> Vec<Constraint> {
        std::mem::take(&mut self.constraints)
    }

    pub(crate) fn push_constraint(&mut self, c: Constraint) {
        self.constraints.push(c);
    }

    pub(crate) fn constraints(&self) -> &[Constraint] {
        &self.constraints
    }

    pub(crate) fn substitutions(&self) -> &HashMap<TyId, Ty> {
        &self.substitutions
    }

    pub(crate) fn substitutions_mut(&mut self) -> &mut HashMap<TyId, Ty> {
        &mut self.substitutions
    }

    pub(crate) fn values_mut(&mut self) -> &mut HashMap<ExprId, ValueResolution> {
        &mut self.values
    }

    pub(crate) fn promotions_mut(&mut self) -> &mut HashMap<ExprId, PromotionInfo> {
        &mut self.promotions
    }

    pub(crate) fn member_kinds_mut(&mut self) -> &mut HashMap<ExprId, MemberKind> {
        &mut self.member_kinds
    }

    pub(crate) fn into_solution(self) -> Solution {
        // Resolve inference variables in ValueResolution substitutions
        // Clone substitutions first since we need it for both resolution and the final solution
        let substitutions = self.substitutions;
        let oracle = self.oracle;
        let resolved_values: HashMap<ExprId, ValueResolution> = self
            .values
            .into_iter()
            .map(|(expr_id, value_res)| {
                let mut resolved_subs = kestrel_semantic_tree::ty::Substitutions::new();
                for (sym_id, ty) in value_res.substitutions.iter() {
                    // Resolve inference variables in the type
                    let resolved_ty = resolve_type_for_solution(ty, &substitutions, oracle);
                    resolved_subs.insert(*sym_id, resolved_ty);
                }
                (
                    expr_id,
                    ValueResolution::new(value_res.symbol_id, resolved_subs),
                )
            })
            .collect();

        let mut solution =
            Solution::with_promotions(substitutions, resolved_values, self.promotions, self.errors);
        solution.member_kinds = self.member_kinds;
        solution
    }
}

/// Resolve a type using the given substitutions (for building the final solution).
fn resolve_type_for_solution(
    ty: &Ty,
    substitutions: &HashMap<TyId, Ty>,
    oracle: &dyn TypeOracle,
) -> Ty {
    match ty.kind() {
        TyKind::Infer => {
            // Look up the inference variable in substitutions
            if let Some(resolved) = substitutions.get(&ty.id()) {
                // Recursively resolve in case the result also has inference vars
                resolve_type_for_solution(resolved, substitutions, oracle)
            } else {
                ty.clone()
            }
        },
        TyKind::Struct {
            symbol,
            substitutions: struct_subs,
        } => {
            // Resolve type arguments in struct substitutions
            let mut resolved_subs = kestrel_semantic_tree::ty::Substitutions::new();
            for (sym_id, arg_ty) in struct_subs.iter() {
                resolved_subs.insert(
                    *sym_id,
                    resolve_type_for_solution(arg_ty, substitutions, oracle),
                );
            }
            Ty::generic_struct(symbol.clone(), resolved_subs, ty.span().clone())
        },
        TyKind::Function {
            params,
            return_type,
        } => {
            let resolved_params: Vec<Ty> = params
                .iter()
                .map(|p| resolve_type_for_solution(p, substitutions, oracle))
                .collect();
            let resolved_return = resolve_type_for_solution(return_type, substitutions, oracle);
            Ty::function(resolved_params, resolved_return, ty.span().clone())
        },
        TyKind::Tuple(elements) => {
            let resolved_elements: Vec<Ty> = elements
                .iter()
                .map(|e| resolve_type_for_solution(e, substitutions, oracle))
                .collect();
            Ty::tuple(resolved_elements, ty.span().clone())
        },
        TyKind::AssociatedType { symbol, container } => {
            // Resolve inference placeholders in the container
            if let Some(container_ty) = container {
                let resolved_container =
                    resolve_type_for_solution(container_ty, substitutions, oracle);
                // If container is no longer an inference placeholder, try to resolve the associated type
                if !matches!(resolved_container.kind(), TyKind::Infer) {
                    // Try to resolve the associated type using the oracle
                    if let Some(resolved_assoc) = oracle.resolve_associated_type(
                        &resolved_container,
                        &symbol.metadata().name().value,
                    ) {
                        // Recursively resolve in case the result has more associated types
                        return resolve_type_for_solution(&resolved_assoc, substitutions, oracle);
                    }
                }
                // If we couldn't fully resolve, at least return with resolved container
                if resolved_container.id() != container_ty.id() {
                    Ty::qualified_associated_type(
                        symbol.clone(),
                        resolved_container,
                        ty.span().clone(),
                    )
                } else {
                    ty.clone()
                }
            } else {
                ty.clone()
            }
        },
        _ => ty.clone(),
    }
}

impl<'a> InferenceContext<'a> {
    pub(crate) fn type_registry(&self) -> &HashMap<TyId, Ty> {
        &self.type_registry
    }

    pub(crate) fn add_error(&mut self, error: InferenceError) {
        self.errors.push(error);
    }

    #[allow(dead_code)]
    pub(crate) fn errors(&self) -> &[InferenceError] {
        &self.errors
    }

    pub(crate) fn closure_metadata(&self) -> &HashMap<TyId, ClosureMetadata> {
        &self.closure_metadata
    }

    /// Mark a TyId as being from a literal (has ExpressibleBy* constraint).
    pub(crate) fn mark_literal_ty(&mut self, ty_id: TyId) {
        self.literal_ty_ids.insert(ty_id);
    }

    /// Check if a TyId is from a literal.
    pub(crate) fn is_literal_ty(&self, ty_id: TyId) -> bool {
        self.literal_ty_ids.contains(&ty_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle::MemberError;
    use kestrel_span::Span;
    use semantic_tree::symbol::SymbolId;

    /// A minimal oracle for testing.
    struct TestOracle;

    impl TypeOracle for TestOracle {
        fn resolve_member(
            &self,
            _receiver_ty: &Ty,
            _member: &str,
            _is_static: bool,
        ) -> Result<crate::oracle::MemberResolution, MemberError> {
            Err(MemberError::NotFound {
                receiver_ty: Ty::unit(Span::new(0, 0..0)),
                member: String::new(),
            })
        }

        fn conforms_to(&self, _ty: &Ty, _protocol_id: SymbolId) -> bool {
            false
        }

        fn resolve_associated_type(&self, _container: &Ty, _assoc_name: &str) -> Option<Ty> {
            None
        }

        fn symbol_name(&self, _symbol_id: SymbolId) -> Option<String> {
            None
        }

        fn builtin_protocol(
            &self,
            _feature: kestrel_semantic_tree::builtins::LanguageFeature,
        ) -> Option<SymbolId> {
            None
        }

        fn default_array_type(&self, _element_ty: Ty, _span: Span) -> Option<Ty> {
            None
        }
    }

    #[test]
    fn test_context_creation() {
        let oracle = TestOracle;
        let ctx = InferenceContext::new(&oracle);
        assert!(ctx.constraints.is_empty());
        assert!(ctx.substitutions.is_empty());
    }

    #[test]
    fn test_add_equality_constraint() {
        let oracle = TestOracle;
        let mut ctx = InferenceContext::new(&oracle);

        let ty1 = Ty::unit(Span::new(0, 0..2));
        let ty2 = Ty::unit(Span::new(0, 3..5));

        ctx.equate(ty1.id(), ty2.id(), Span::new(0, 0..5));

        assert_eq!(ctx.constraints.len(), 1);
    }
}
