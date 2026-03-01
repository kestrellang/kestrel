use semantic_tree::symbol::{Symbol, SymbolId};
use std::collections::HashMap;

use crate::language::KestrelLanguage;

use super::Ty;

/// Maps type parameter SymbolIds to their substituted types.
///
/// This is used when instantiating generic types, e.g., `List[Int]` creates
/// a Substitutions mapping the `T` parameter's SymbolId to `Int`.
#[derive(Debug, Clone, Default)]
pub struct Substitutions {
    map: HashMap<SymbolId, Ty>,
}

impl Substitutions {
    /// Create an empty substitution map
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Check if there are no substitutions
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Get the number of substitutions
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Insert a type substitution for a type parameter
    pub fn insert(&mut self, param_id: SymbolId, ty: Ty) {
        self.map.insert(param_id, ty);
    }

    /// Get the substituted type for a type parameter
    pub fn get(&self, param_id: SymbolId) -> Option<&Ty> {
        self.map.get(&param_id)
    }

    /// Check if a type parameter has a substitution
    pub fn contains(&self, param_id: SymbolId) -> bool {
        self.map.contains_key(&param_id)
    }

    /// Iterate over all substitutions
    pub fn iter(&self) -> impl Iterator<Item = (&SymbolId, &Ty)> {
        self.map.iter()
    }

    /// Iterate over just the types (values).
    /// WARNING: This iterates in HashMap order which is non-deterministic!
    /// For generic function calls, use `types_in_order` instead.
    pub fn types(&self) -> impl Iterator<Item = &Ty> {
        self.map.values()
    }

    /// Get the substituted types in the order specified by the given type parameter IDs.
    /// Returns types for each parameter ID in order, or None if any parameter is not found.
    pub fn types_in_order(&self, param_ids: &[SymbolId]) -> Option<Vec<&Ty>> {
        param_ids.iter().map(|id| self.get(*id)).collect()
    }

    /// Apply a function to each substitution value, returning a new map.
    pub fn map_values(&self, f: &mut impl FnMut(&Ty) -> Ty) -> Substitutions {
        let mut result = Substitutions::new();
        for (id, ty) in self.iter() {
            result.insert(*id, f(ty));
        }
        result
    }

    /// Apply substitutions to a type, replacing any type parameters with their
    /// substituted types. Returns a new type with substitutions applied.
    pub fn apply(&self, ty: &Ty) -> Ty {
        use std::collections::HashSet;
        self.apply_with_visited(ty, &mut HashSet::new())
    }

    /// Internal helper for apply that tracks visited type parameters to detect cycles
    fn apply_with_visited(&self, ty: &Ty, visited: &mut std::collections::HashSet<SymbolId>) -> Ty {
        use super::TyKind;

        match ty.kind() {
            // Type parameter - look up in substitutions
            TyKind::TypeParameter(param_symbol) => {
                let param_id = Symbol::<KestrelLanguage>::metadata(param_symbol.as_ref()).id();
                let param_name = Symbol::<KestrelLanguage>::metadata(param_symbol.as_ref())
                    .name()
                    .value
                    .clone();

                // Debug logging for Rhs substitution
                if param_name == "Rhs" {
                    debug_trace!(
                        "  === Substituting type parameter {} (ID: {:?}) ===",
                        param_name,
                        param_id
                    );
                    debug_trace!("  Available substitutions:");
                    for (id, ty) in self.iter() {
                        debug_trace!("    {:?} -> {:?}", id, ty);
                    }
                }

                // Check if we're already visiting this type parameter (cycle detected)
                if visited.contains(&param_id) {
                    // Cycle detected - return the type parameter as-is to break the cycle
                    return ty.clone();
                }

                if let Some(substituted) = self.get(param_id) {
                    if param_name == "Rhs" {
                        debug_trace!("  Found substitution: {:?}", substituted);
                    }
                    // Mark this parameter as being visited
                    visited.insert(param_id);
                    // Recursively apply in case the substituted type also has type params
                    let result = self.apply_with_visited(substituted, visited);
                    // Remove from visited set after processing
                    visited.remove(&param_id);
                    result
                } else {
                    if param_name == "Rhs" {
                        debug_trace!(
                            "  No substitution found for {} (ID: {:?})",
                            param_name,
                            param_id
                        );
                    }
                    // No substitution found, return as-is
                    ty.clone()
                }
            },

            // All other types: recurse into children
            _ => ty.map_children(&mut |child| self.apply_with_visited(child, visited)),
        }
    }

    /// Check if this substitution map is a specialization of another.
    ///
    /// This substitution map is a specialization of the pattern if for every
    /// type parameter, the type in this map is a specialization of the type
    /// in the pattern map.
    pub fn is_specialization_of(&self, pattern: &Substitutions) -> bool {
        if self.len() != pattern.len() {
            return false;
        }

        for (id, pattern_ty) in pattern.iter() {
            let self_ty = match self.get(*id) {
                Some(ty) => ty,
                None => return false,
            };

            if !self_ty.is_specialization_of(pattern_ty) {
                return false;
            }
        }

        true
    }

    /// Check if this substitution map overlaps with another.
    ///
    /// Two substitution maps overlap if there exists a substitution map
    /// that is a specialization of both.
    pub fn overlaps_with(&self, other: &Substitutions) -> bool {
        if self.len() != other.len() {
            return false;
        }

        for (id, ty1) in self.iter() {
            let ty2 = match other.get(*id) {
                Some(ty) => ty,
                None => return false,
            };

            if !ty1.overlaps_with(ty2) {
                return false;
            }
        }

        true
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_span::Span;

    #[test]
    fn test_empty_substitutions() {
        let subs = Substitutions::new();
        assert!(subs.is_empty());
        assert_eq!(subs.len(), 0);
    }

    #[test]
    fn test_insert_and_get() {
        let mut subs = Substitutions::new();
        let id = SymbolId::new();
        let ty = Ty::unit(Span::new(0, 0..2));

        subs.insert(id, ty);

        assert!(!subs.is_empty());
        assert_eq!(subs.len(), 1);
        assert!(subs.contains(id));
        assert!(subs.get(id).unwrap().is_unit());
    }

    #[test]
    fn test_apply_to_base_type() {
        let subs = Substitutions::new();
        let ty = Ty::int(super::super::IntBits::I32, Span::new(0, 0..3));

        let result = subs.apply(&ty);
        assert!(result.is_int());
    }
}
