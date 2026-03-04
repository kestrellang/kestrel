//! ProtocolConformancesForType query — cached protocol conformance collection.
//!
//! Wraps `collect_protocol_conformances_for_type` to cache the BFS over the
//! protocol conformance graph for a type. Called from member resolution (8 sites).

use std::hash::{Hash, Hasher};

use kestrel_semantic_tree::ty::Ty;

use crate::SemanticModel;
use crate::query::Query;
use crate::ty_cache_key::TyCacheKey;
use crate::type_oracle::collect_protocol_conformances_for_type;

/// Query: what protocols does `ty` conform to (including via extensions and inheritance)?
///
/// Returns protocol `Ty` values with substitutions applied.
#[derive(Clone)]
pub struct ProtocolConformancesForType {
    pub ty: Ty,
    cache_key: TyCacheKey,
}

impl ProtocolConformancesForType {
    pub fn new(ty: &Ty) -> Self {
        Self {
            ty: ty.clone(),
            cache_key: TyCacheKey::from_ty(ty),
        }
    }
}

impl Hash for ProtocolConformancesForType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cache_key.hash(state);
    }
}

impl PartialEq for ProtocolConformancesForType {
    fn eq(&self, other: &Self) -> bool {
        self.cache_key == other.cache_key
    }
}

impl Eq for ProtocolConformancesForType {}

impl Query for ProtocolConformancesForType {
    type Output = Vec<Ty>;

    fn execute(self, model: &SemanticModel) -> Vec<Ty> {
        collect_protocol_conformances_for_type(model, &self.ty)
    }
}
