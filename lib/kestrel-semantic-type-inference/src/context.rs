//! Inference context for collecting and solving constraints.
//!
//! The [`InferenceContext`] is the main entry point for type inference.
//! It collects constraints during expression resolution and then solves
//! them to produce a [`Solution`].

use std::collections::HashMap;

use kestrel_semantic_tree::expr::ExprId;
use kestrel_semantic_tree::ty::{Ty, TyId};
use kestrel_span::Span;

use crate::constraint::{Constraint, ProtocolRef};
use crate::error::InferenceError;
use crate::oracle::TypeOracle;
use crate::solution::{Solution, ValueResolution};

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
        }
    }

    /// Register a type for tracking during inference.
    ///
    /// This should be called for any type that might be unified or
    /// referenced in constraints. It allows looking up the original
    /// type from its ID.
    pub fn register_type(&mut self, ty: &Ty) {
        self.type_registry.insert(ty.id(), ty.clone());
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

    // === Solving ===

    /// Solve all collected constraints and return a solution.
    ///
    /// This consumes the context and returns either a [`Solution`] containing
    /// all resolved types and values, or an [`InferenceError`] if the constraints
    /// cannot be satisfied.
    pub fn solve(self) -> Result<Solution, InferenceError> {
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
        Solution::with_mappings(self.substitutions, self.values)
    }

    pub(crate) fn type_registry(&self) -> &HashMap<TyId, Ty> {
        &self.type_registry
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
