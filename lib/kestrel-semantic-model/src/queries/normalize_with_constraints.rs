//! NormalizeWithConstraints query — cached type normalization using where-clause constraints.
//!
//! Wraps `normalize_type_with_context` so repeated normalizations of the same
//! type in the same context return cached results.

use std::hash::{Hash, Hasher};

use semantic_tree::symbol::SymbolId;
use kestrel_semantic_tree::ty::Ty;

use crate::SemanticModel;
use crate::query::Query;
use crate::ty_cache_key::TyCacheKey;
use crate::type_oracle::normalize_type_with_context;

/// Query: normalize a type using where-clause equality constraints in a given context.
#[derive(Clone)]
pub struct NormalizeWithConstraints {
    pub ty: Ty,
    pub context_id: SymbolId,
    cache_key: TyCacheKey,
}

impl NormalizeWithConstraints {
    pub fn new(ty: &Ty, context_id: SymbolId) -> Self {
        Self {
            ty: ty.clone(),
            context_id,
            cache_key: TyCacheKey::from_ty(ty),
        }
    }
}

impl Hash for NormalizeWithConstraints {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cache_key.hash(state);
        self.context_id.hash(state);
    }
}

impl PartialEq for NormalizeWithConstraints {
    fn eq(&self, other: &Self) -> bool {
        self.cache_key == other.cache_key && self.context_id == other.context_id
    }
}

impl Eq for NormalizeWithConstraints {}

impl Query for NormalizeWithConstraints {
    type Output = Ty;

    fn execute(self, model: &SemanticModel) -> Ty {
        normalize_type_with_context(model, &self.ty, self.context_id)
    }
}
