//! ResolveAssociatedType query — cached associated type resolution.
//!
//! Wraps `TypeOracle::resolve_associated_type` so repeated lookups of the same
//! container type + associated type name return cached results.

use std::hash::{Hash, Hasher};

use kestrel_semantic_tree::ty::Ty;

use crate::SemanticModel;
use crate::query::Query;
use crate::ty_cache_key::TyCacheKey;
use crate::type_oracle::resolve_associated_type_impl;

/// Query: resolve `container.assoc_name` to a concrete type.
#[derive(Clone)]
pub struct ResolveAssociatedTypeQuery {
    pub container: Ty,
    pub assoc_name: String,
    cache_key: TyCacheKey,
}

impl ResolveAssociatedTypeQuery {
    pub fn new(container: &Ty, assoc_name: &str) -> Self {
        Self {
            container: container.clone(),
            assoc_name: assoc_name.to_string(),
            cache_key: TyCacheKey::from_ty(container),
        }
    }
}

impl Hash for ResolveAssociatedTypeQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cache_key.hash(state);
        self.assoc_name.hash(state);
    }
}

impl PartialEq for ResolveAssociatedTypeQuery {
    fn eq(&self, other: &Self) -> bool {
        self.cache_key == other.cache_key && self.assoc_name == other.assoc_name
    }
}

impl Eq for ResolveAssociatedTypeQuery {}

impl Query for ResolveAssociatedTypeQuery {
    type Output = Option<Ty>;

    fn execute(self, model: &SemanticModel) -> Option<Ty> {
        resolve_associated_type_impl(model, &self.container, &self.assoc_name)
    }
}
