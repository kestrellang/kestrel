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

    /// Iterate over just the types (values)
    pub fn types(&self) -> impl Iterator<Item = &Ty> {
        self.map.values()
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

                // Check if we're already visiting this type parameter (cycle detected)
                if visited.contains(&param_id) {
                    // Cycle detected - return the type parameter as-is to break the cycle
                    return ty.clone();
                }

                if let Some(substituted) = self.get(param_id) {
                    // Mark this parameter as being visited
                    visited.insert(param_id);
                    // Recursively apply in case the substituted type also has type params
                    let result = self.apply_with_visited(substituted, visited);
                    // Remove from visited set after processing
                    visited.remove(&param_id);
                    result
                } else {
                    // No substitution found, return as-is
                    ty.clone()
                }
            }

            // Composite types - recursively apply to components
            TyKind::Tuple(elements) => {
                let new_elements: Vec<Ty> = elements
                    .iter()
                    .map(|e| self.apply_with_visited(e, visited))
                    .collect();
                Ty::tuple(new_elements, ty.span().clone())
            }

            TyKind::Array(element_type) => {
                let new_element = self.apply_with_visited(element_type, visited);
                Ty::array(new_element, ty.span().clone())
            }

            TyKind::Function {
                params,
                return_type,
            } => {
                let new_params: Vec<Ty> = params
                    .iter()
                    .map(|p| self.apply_with_visited(p, visited))
                    .collect();
                let new_return = self.apply_with_visited(return_type, visited);
                Ty::function(new_params, new_return, ty.span().clone())
            }

            // Instantiated types - recursively apply to their substitutions
            TyKind::Struct {
                symbol,
                substitutions,
            } => {
                let new_subs = self.apply_to_substitutions_with_visited(substitutions, visited);
                Ty::generic_struct(symbol.clone(), new_subs, ty.span().clone())
            }

            TyKind::Protocol {
                symbol,
                substitutions,
            } => {
                let new_subs = self.apply_to_substitutions_with_visited(substitutions, visited);
                Ty::generic_protocol(symbol.clone(), new_subs, ty.span().clone())
            }

            TyKind::TypeAlias {
                symbol,
                substitutions,
            } => {
                let new_subs = self.apply_to_substitutions_with_visited(substitutions, visited);
                Ty::generic_type_alias(symbol.clone(), new_subs, ty.span().clone())
            }

            // Associated type - apply substitutions to container if present
            TyKind::AssociatedType { symbol, container } => match container {
                Some(container_ty) => {
                    let new_container = self.apply_with_visited(container_ty, visited);
                    Ty::qualified_associated_type(symbol.clone(), new_container, ty.span().clone())
                }
                None => ty.clone(),
            },

            // Base types and special types - return as-is
            TyKind::Unit
            | TyKind::Never
            | TyKind::Int(_)
            | TyKind::Float(_)
            | TyKind::Bool
            | TyKind::String
            | TyKind::Error
            | TyKind::SelfType
            | TyKind::Infer => ty.clone(),
        }
    }

    /// Internal helper for apply_to_substitutions that tracks visited type parameters
    fn apply_to_substitutions_with_visited(
        &self,
        other: &Substitutions,
        visited: &mut std::collections::HashSet<SymbolId>,
    ) -> Substitutions {
        let mut result = Substitutions::new();
        for (id, ty) in other.iter() {
            result.insert(*id, self.apply_with_visited(ty, visited));
        }
        result
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
        let ty = Ty::unit(Span::from(0..2));

        subs.insert(id, ty);

        assert!(!subs.is_empty());
        assert_eq!(subs.len(), 1);
        assert!(subs.contains(id));
        assert!(subs.get(id).unwrap().is_unit());
    }

    #[test]
    fn test_apply_to_base_type() {
        let subs = Substitutions::new();
        let ty = Ty::int(super::super::IntBits::I32, Span::from(0..3));

        let result = subs.apply(&ty);
        assert!(result.is_int());
    }
}
