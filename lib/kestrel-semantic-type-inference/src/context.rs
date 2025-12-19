//! Inference context for collecting and solving constraints.
//!
//! The [`InferenceContext`] is the main entry point for type inference.
//! It collects constraints during expression resolution and then solves
//! them to produce a [`Solution`].

use std::collections::HashMap;

use kestrel_semantic_tree::expr::ExprId;
use kestrel_semantic_tree::ty::{ParamInfo, Ty, TyId, TyKind};
use kestrel_span::Span;

use crate::constraint::{Constraint, ProtocolRef};
use crate::error::InferenceError;
use crate::oracle::TypeOracle;
use crate::solution::{Solution, ValueResolution};

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
    /// Map from TyId to its original Ty (for looking up spans and kinds)
    type_registry: HashMap<TyId, Ty>,
    /// Accumulated errors during solving
    errors: Vec<InferenceError>,
    /// Closure metadata for error reporting (TyId -> ClosureMetadata)
    closure_metadata: HashMap<TyId, ClosureMetadata>,
    /// The expected return type for the current function/method body
    return_type: Option<Ty>,
}

impl<'a> InferenceContext<'a> {
    /// Create a new inference context with the given oracle.
    pub fn new(oracle: &'a dyn TypeOracle) -> Self {
        Self {
            oracle,
            constraints: Vec::new(),
            substitutions: HashMap::new(),
            values: HashMap::new(),
            type_registry: HashMap::new(),
            errors: Vec::new(),
            closure_metadata: HashMap::new(),
            return_type: None,
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
            TyKind::Array(elem_ty) => {
                self.register_type(elem_ty);
            }
            TyKind::Tuple(elem_tys) => {
                for elem_ty in elem_tys {
                    self.register_type(elem_ty);
                }
            }
            TyKind::Function {
                params,
                return_type,
            } => {
                for param_ty in params {
                    self.register_type(param_ty);
                }
                self.register_type(return_type);
            }
            TyKind::Struct { substitutions, .. }
            | TyKind::Protocol { substitutions, .. }
            | TyKind::TypeAlias { substitutions, .. } => {
                for (_, sub_ty) in substitutions.iter() {
                    self.register_type(sub_ty);
                }
            }
            TyKind::AssociatedType { container, .. } => {
                if let Some(container_ty) = container {
                    self.register_type(container_ty);
                }
            }
            TyKind::UnresolvedFunction {
                param_info,
                return_type,
            } => {
                self.register_type(return_type);
                match param_info {
                    ParamInfo::ImplicitIt { it_type } => {
                        self.register_type(it_type);
                    }
                    ParamInfo::Explicit { param_types } => {
                        for pt in param_types {
                            self.register_type(pt);
                        }
                    }
                    ParamInfo::Unconstrained => {}
                }
            }
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
            | TyKind::TypeParameter(_) => {}
        }
    }

    /// Get a registered type by ID.
    pub fn get_type(&self, id: TyId) -> Option<&Ty> {
        // First check substitutions, then registry
        self.substitutions.get(&id).or_else(|| self.type_registry.get(&id))
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
    pub fn member_access(
        &mut self,
        receiver: TyId,
        member: String,
        is_static: bool,
        result: TyId,
        expr_id: ExprId,
        span: Span,
    ) {
        self.constraints.push(Constraint::member_access(
            receiver, member, is_static, result, expr_id, span,
        ));
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

    pub(crate) fn into_solution(self) -> Solution {
        Solution::with_errors(self.substitutions, self.values, self.errors)
    }

    pub(crate) fn type_registry(&self) -> &HashMap<TyId, Ty> {
        &self.type_registry
    }

    pub(crate) fn add_error(&mut self, error: InferenceError) {
        self.errors.push(error);
    }

    pub(crate) fn errors(&self) -> &[InferenceError] {
        &self.errors
    }

    pub(crate) fn closure_metadata(&self) -> &HashMap<TyId, ClosureMetadata> {
        &self.closure_metadata
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
                receiver_ty: Ty::unit(Span::from(0..0)),
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

        let ty1 = Ty::unit(Span::from(0..2));
        let ty2 = Ty::unit(Span::from(3..5));

        ctx.equate(ty1.id(), ty2.id(), Span::from(0..5));

        assert_eq!(ctx.constraints.len(), 1);
    }
}
