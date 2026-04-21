//! ConformsTo query — cached conformance checking.
//!
//! Wraps the `conforms_to` TypeOracle method so repeated calls with the same
//! type and protocol return cached results. This is the highest-impact cache
//! (27+ call sites per compilation).

use std::hash::{Hash, Hasher};

use kestrel_semantic_tree::builtins::{BuiltinKind, LanguageFeature};
use kestrel_semantic_tree::ty::{Ty, TyKind};
use kestrel_semantic_type_inference::TypeOracle;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::{ConformancesForSymbol, ExtensionsFor, TypeParameterBounds};
use crate::query::Query;
use crate::ty_cache_key::TyCacheKey;
use crate::type_oracle::{
    bound_protocols_include, check_transitive_conformance_impl,
    filter_applicable_extensions_for_conformance, get_type_substitutions, get_type_symbol_id,
};

/// Query: does `ty` conform to the protocol identified by `protocol_id`?
///
/// Hash/Eq use the `TyCacheKey` (not the `Ty` directly, since `Ty` isn't hashable).
#[derive(Clone)]
pub struct ConformsToQuery {
    pub ty: Ty,
    pub protocol_id: SymbolId,
    cache_key: TyCacheKey,
}

impl ConformsToQuery {
    pub fn new(ty: &Ty, protocol_id: SymbolId) -> Self {
        Self {
            ty: ty.clone(),
            protocol_id,
            cache_key: TyCacheKey::from_ty(ty),
        }
    }
}

impl Hash for ConformsToQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cache_key.hash(state);
        self.protocol_id.hash(state);
    }
}

impl PartialEq for ConformsToQuery {
    fn eq(&self, other: &Self) -> bool {
        self.cache_key == other.cache_key && self.protocol_id == other.protocol_id
    }
}

impl Eq for ConformsToQuery {}

impl Query for ConformsToQuery {
    type Output = bool;

    fn execute(self, model: &SemanticModel) -> bool {
        conforms_to_impl(model, &self.ty, self.protocol_id)
    }
}

/// Core conformance-checking logic extracted from `impl TypeOracle for SemanticModel`.
///
/// This is the model-level (context-free) conformance check. The `ContextualOracle`
/// handles SelfType and extension bounds separately, then delegates here.
pub(crate) fn conforms_to_impl(model: &SemanticModel, ty: &Ty, protocol_id: SymbolId) -> bool {
    // Handle inference placeholders - can't check conformance yet
    if matches!(ty.kind(), TyKind::Infer) {
        return false;
    }

    // Handle error types - treat as conforming to suppress cascading errors
    if matches!(ty.kind(), TyKind::Error) {
        return true;
    }

    // Handle never type - the bottom type conforms to any protocol
    if matches!(ty.kind(), TyKind::Never) {
        return true;
    }

    // Handle type parameters - check if any bound matches the protocol
    if let TyKind::TypeParameter(type_param) = ty.kind() {
        let bounds = model.query(TypeParameterBounds {
            param_id: type_param.metadata().id(),
        });
        if bound_protocols_include(model, &bounds, protocol_id) {
            return true;
        }
        return false;
    }

    // Handle associated types - check bounds on the associated type definition
    if let TyKind::AssociatedType { symbol, container } = ty.kind() {
        // If the associated type can be resolved to a concrete type, defer to that
        let assoc_name = symbol.metadata().name().value.clone();
        if let Some(container) = container
            && let Some(resolved) =
                kestrel_semantic_type_inference::TypeOracle::resolve_associated_type(
                    model,
                    container,
                    &assoc_name,
                )
        {
            return model.query(ConformsToQuery::new(&resolved, protocol_id));
        }

        if let Some(bounds) = symbol.bounds()
            && bound_protocols_include(model, &bounds, protocol_id)
        {
            return true;
        }
        return false;
    }

    // Expand type aliases before checking conformance.
    let ty = &ty.expand_aliases();

    // Handle tuple types - check if protocol has tuple_conformance_propagation flag
    if let TyKind::Tuple(elements) = ty.kind() {
        if let Some(feature) = model.builtin_registry().protocol_feature(protocol_id) {
            let definition = feature.definition();
            if let BuiltinKind::Protocol {
                tuple_conformance_propagation: true,
                ..
            } = definition.kind
            {
                return elements
                    .iter()
                    .all(|elem| model.query(ConformsToQuery::new(elem, protocol_id)));
            }
        }
        return false;
    }

    // Handle FFISafe conformance for primitive machine types.
    if let Some(ffi_safe_id) = model.builtin_protocol(LanguageFeature::FFISafe)
        && protocol_id == ffi_safe_id
    {
        match ty.kind() {
            TyKind::Int(_) | TyKind::Float(_) | TyKind::Bool | TyKind::String => {
                return true;
            },
            TyKind::Pointer(pointee) => {
                return model.query(ConformsToQuery::new(pointee, protocol_id));
            },
            _ => {},
        }
    }

    // Handle primitive types - they implicitly conform to their literal protocols
    match ty.kind() {
        TyKind::Int(_) => {
            if let Some(lit_protocol_id) =
                model.builtin_protocol(LanguageFeature::ExpressibleByIntLiteral)
                && protocol_id == lit_protocol_id
            {
                return true;
            }
            return false;
        },
        TyKind::Float(_) => {
            if let Some(lit_protocol_id) =
                model.builtin_protocol(LanguageFeature::ExpressibleByFloatLiteral)
                && protocol_id == lit_protocol_id
            {
                return true;
            }
            if let Some(lit_protocol_id) =
                model.builtin_protocol(LanguageFeature::ExpressibleByIntLiteral)
                && protocol_id == lit_protocol_id
            {
                return true;
            }
            return false;
        },
        TyKind::Bool => {
            if let Some(lit_protocol_id) =
                model.builtin_protocol(LanguageFeature::ExpressibleByBoolLiteral)
                && protocol_id == lit_protocol_id
            {
                return true;
            }
            return false;
        },
        TyKind::String => {
            if let Some(lit_protocol_id) =
                model.builtin_protocol(LanguageFeature::ExpressibleByStringLiteral)
                && protocol_id == lit_protocol_id
            {
                return true;
            }
            return false;
        },
        _ => {},
    }

    // Get the type's symbol ID to check conformances
    let type_symbol_id = match get_type_symbol_id(ty) {
        Some(id) => id,
        None => return false,
    };

    // Get all conformances for this type
    let conformances = model.query(ConformancesForSymbol {
        symbol_id: type_symbol_id,
    });

    // Check if any conformance matches the protocol
    for conformance in conformances {
        if let TyKind::Protocol { symbol, .. } = conformance.kind()
            && symbol.metadata().id() == protocol_id
        {
            return true;
        }
    }

    // Also check extensions for conformances
    let actual_subs = get_type_substitutions(ty);

    let extensions = model.query(ExtensionsFor {
        target_id: type_symbol_id,
    });

    let applicable_extensions =
        filter_applicable_extensions_for_conformance(Some(model), &extensions, &actual_subs);

    for extension in &applicable_extensions {
        let ext_conformances = model.query(ConformancesForSymbol {
            symbol_id: extension.metadata().id(),
        });

        for conformance in ext_conformances {
            if let TyKind::Protocol { symbol, .. } = conformance.kind()
                && symbol.metadata().id() == protocol_id
            {
                return true;
            }
        }
    }

    // Check transitive conformance through protocol extensions.
    let mut visited = std::collections::HashSet::new();
    if check_transitive_conformance_impl(
        model,
        ty,
        protocol_id,
        type_symbol_id,
        &applicable_extensions,
        &mut visited,
    ) {
        return true;
    }

    false
}
