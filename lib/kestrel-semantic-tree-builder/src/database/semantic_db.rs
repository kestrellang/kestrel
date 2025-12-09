//! Semantic database implementation
//!
//! The `SemanticDatabase` provides the main query interface for semantic analysis,
//! with caching for expensive queries like scope computation.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use kestrel_prelude::primitives;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use kestrel_semantic_tree::error::ModuleNotFoundError;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{FloatBits, IntBits, Ty};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::resolution::VisibilityChecker;

use super::queries::{
    get_import_data, Db, Import, ImportItem, Scope, SymbolResolution, TypePathResolution,
    ValuePathResolution,
};
use super::registry::SymbolRegistry;

/// Resolve a primitive type name to its semantic type
fn resolve_primitive_type(name: &str, span: kestrel_span::Span) -> Option<Ty> {
    match name {
        primitives::INT => Some(Ty::int(IntBits::I64, span)),
        primitives::I8 => Some(Ty::int(IntBits::I8, span)),
        primitives::I16 => Some(Ty::int(IntBits::I16, span)),
        primitives::I32 => Some(Ty::int(IntBits::I32, span)),
        primitives::I64 => Some(Ty::int(IntBits::I64, span)),
        primitives::FLOAT => Some(Ty::float(FloatBits::F64, span)),
        primitives::F32 => Some(Ty::float(FloatBits::F32, span)),
        primitives::F64 => Some(Ty::float(FloatBits::F64, span)),
        primitives::BOOL => Some(Ty::bool(span)),
        primitives::STRING => Some(Ty::string(span)),
        primitives::SELF_TYPE => Some(Ty::self_type(span)),
        _ => None,
    }
}

/// Database for semantic queries with caching
pub struct SemanticDatabase {
    registry: SymbolRegistry,
    scope_cache: RwLock<HashMap<SymbolId, Arc<Scope>>>,
}

impl SemanticDatabase {
    /// Create a new database with the given symbol registry
    pub fn new(registry: SymbolRegistry) -> Self {
        Self {
            registry,
            scope_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Get the symbol registry
    pub fn registry(&self) -> &SymbolRegistry {
        &self.registry
    }

    /// Compute scope for a symbol
    fn compute_scope(&self, symbol_id: SymbolId) -> Arc<Scope> {
        let symbol = self.symbol_by_id(symbol_id).expect("symbol must exist");

        // Get imports using imports_in_scope query
        let imports_data = self.imports_in_scope(symbol_id);
        let mut imports = HashMap::new();

        for import in imports_data.iter() {
            // Process specific import items
            for item in &import.items {
                if let Some(target_id) = item.target_id {
                    let name = item.alias.as_ref().unwrap_or(&item.name);
                    imports
                        .entry(name.clone())
                        .or_insert_with(Vec::new)
                        .push(target_id);
                }
            }

            // Handle whole-module imports
            if import.items.is_empty() {
                if let Some(alias) = &import.alias {
                    // import A.B.C as D
                    if let Ok(module_id) =
                        self.resolve_module_path(import.module_path.clone(), symbol_id)
                    {
                        imports
                            .entry(alias.clone())
                            .or_insert_with(Vec::new)
                            .push(module_id);
                    }
                } else {
                    // import A.B.C → import all visible symbols
                    if let Ok(module_id) =
                        self.resolve_module_path(import.module_path.clone(), symbol_id)
                    {
                        let module_scope = self.scope_for(module_id);

                        // Import all visible declarations from module
                        for (name, ids) in &module_scope.declarations {
                            for &decl_id in ids {
                                if self.is_visible_from(decl_id, symbol_id) {
                                    imports
                                        .entry(name.clone())
                                        .or_insert_with(Vec::new)
                                        .push(decl_id);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Get declarations (children that aren't imports)
        let declarations = symbol
            .metadata()
            .children()
            .into_iter()
            .filter(|c| !matches!(c.metadata().kind(), KestrelSymbolKind::Import))
            .fold(HashMap::new(), |mut map, child| {
                map.entry(child.metadata().name().value.clone())
                    .or_insert_with(Vec::new)
                    .push(child.metadata().id());
                map
            });

        Arc::new(Scope {
            symbol_id,
            imports,
            declarations,
            parent: symbol.metadata().parent().map(|p| p.metadata().id()),
        })
    }

    /// Helper to extract value information from resolved symbols
    fn extract_value_from_symbols(
        &self,
        symbols: &[Arc<dyn Symbol<KestrelLanguage>>],
        segment: &str,
        index: usize,
    ) -> ValuePathResolution {
        if symbols.is_empty() {
            return ValuePathResolution::NotFound {
                segment: segment.to_string(),
                index,
            };
        }

        // Check if all symbols are functions (potential overloads)
        let all_functions = symbols
            .iter()
            .all(|s| s.metadata().kind() == KestrelSymbolKind::Function);

        if all_functions && symbols.len() > 1 {
            return ValuePathResolution::Overloaded {
                candidates: symbols.iter().map(|s| s.metadata().id()).collect(),
            };
        }

        // Single symbol - try to extract value
        let symbol = &symbols[0];

        // Check for ValueBehavior
        if let Some(value_beh) = symbol.value_behavior() {
            return ValuePathResolution::Symbol {
                symbol_id: symbol.metadata().id(),
                ty: value_beh.ty().clone(),
            };
        }

        // Check for CallableBehavior (functions are values)
        if let Some(callable_beh) = symbol.callable_behavior() {
            return ValuePathResolution::Symbol {
                symbol_id: symbol.metadata().id(),
                ty: callable_beh.function_type(),
            };
        }

        ValuePathResolution::NotAValue {
            symbol_id: symbol.metadata().id(),
        }
    }

    /// Search for a name in inherited protocols (for associated type inheritance)
    fn find_in_inherited_protocols(
        &self,
        protocol: &Arc<dyn Symbol<KestrelLanguage>>,
        name: &str,
    ) -> Option<SymbolResolution> {
        use kestrel_semantic_tree::ty::TyKind;

        let conformances_beh = protocol.conformances_behavior()?;

        for parent_ty in conformances_beh.conformances() {
            if let TyKind::Protocol {
                symbol: parent_proto,
                ..
            } = parent_ty.kind()
            {
                // Check direct children of parent protocol
                let parent_dyn = parent_proto.clone() as Arc<dyn Symbol<KestrelLanguage>>;
                for child in parent_dyn.metadata().children() {
                    if child.metadata().kind() == KestrelSymbolKind::AssociatedType
                        && child.metadata().name().value == name
                    {
                        return Some(SymbolResolution::Found(vec![child.metadata().id()]));
                    }
                }

                // Recursively check grandparent protocols
                if let Some(result) = self.find_in_inherited_protocols(&parent_dyn, name) {
                    return Some(result);
                }
            }
        }

        None
    }

    /// Resolve an associated type from a type parameter's protocol bounds.
    ///
    /// Given a type parameter T and a segment "Item", this looks up the where clause
    /// bounds for T (e.g., `where T: Iterator`) and finds the associated type "Item"
    /// from those protocol bounds.
    ///
    /// The context_id is the symbol where the type is being resolved (usually the function
    /// that owns the type parameter). We use this instead of the type parameter's parent
    /// because the parent may not be set correctly during the build phase.
    ///
    /// Returns None if no associated type is found, allowing the caller to fall back
    /// to normal child lookup (which will fail with a proper error message).
    fn resolve_associated_type_from_type_param_with_context(
        &self,
        type_param: &Arc<TypeParameterSymbol>,
        segment: &str,
        remaining_path: &[String],
        _index: usize,
        context_id: SymbolId,
    ) -> Option<TypePathResolution> {
        use kestrel_semantic_tree::ty::TyKind;

        // Get the context symbol (the function/struct where this type is being resolved)
        let context = self.symbol_by_id(context_id)?;

        // Get the where clause from the context's GenericsBehavior
        let generics_beh = context.generics_behavior()?;
        let where_clause = generics_beh.where_clause();

        // Get protocol bounds for this type parameter
        let param_id = type_param.metadata().id();
        let bounds = where_clause.bounds_for(param_id);

        if bounds.is_empty() {
            return None;
        }

        // Search protocol bounds for the associated type
        for bound in bounds {
            if let TyKind::Protocol { symbol: protocol, .. } = bound.kind() {
                // Check direct children of protocol
                let protocol_dyn = protocol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
                for child in protocol_dyn.metadata().children() {
                    if child.metadata().kind() == KestrelSymbolKind::AssociatedType
                        && child.metadata().name().value == segment
                    {
                        // Found it! Create a qualified associated type
                        if let Some(symbol) = self.symbol_by_id(child.metadata().id()) {
                            if let Ok(assoc_type_arc) = symbol.into_any_arc().downcast::<AssociatedTypeSymbol>() {
                                let span = type_param.metadata().span().clone();
                                let container_ty = Ty::type_parameter(type_param.clone(), span.clone());

                                // If there are more segments (e.g., T.Iter.Item), we need to handle
                                // nested associated types - for now just handle one level
                                if remaining_path.len() > 1 {
                                    // For nested paths like C.Iter.Item, we need to recursively resolve
                                    // First create T.Iter, then look up Item on that
                                    let first_assoc_ty = Ty::qualified_associated_type(
                                        assoc_type_arc.clone(),
                                        container_ty.clone(),
                                        span.clone(),
                                    );

                                    // Now we need to find "Item" in the bounds of "Iter"
                                    // Check if the associated type has bounds that are protocols
                                    if let Some(result) = self.resolve_nested_associated_type(
                                        &assoc_type_arc,
                                        first_assoc_ty,
                                        &remaining_path[1..],
                                    ) {
                                        return Some(result);
                                    }
                                }

                                let ty = Ty::qualified_associated_type(assoc_type_arc, container_ty, span);
                                return Some(TypePathResolution::Resolved(ty));
                            }
                        }
                    }
                }

                // Check inherited protocols
                if let Some(SymbolResolution::Found(ids)) = self.find_in_inherited_protocols(&protocol_dyn, segment) {
                    if let Some(id) = ids.first() {
                        if let Some(symbol) = self.symbol_by_id(*id) {
                            if let Ok(assoc_type_arc) = symbol.into_any_arc().downcast::<AssociatedTypeSymbol>() {
                                let span = type_param.metadata().span().clone();
                                let container_ty = Ty::type_parameter(type_param.clone(), span.clone());
                                let ty = Ty::qualified_associated_type(assoc_type_arc, container_ty, span);
                                return Some(TypePathResolution::Resolved(ty));
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Resolve nested associated types (e.g., C.Iter.Item)
    ///
    /// Given an associated type like "Iter" with bounds "Iterator", and remaining path ["Item"],
    /// find the "Item" associated type from the Iterator protocol bound.
    fn resolve_nested_associated_type(
        &self,
        assoc_type: &Arc<AssociatedTypeSymbol>,
        container_ty: Ty,
        remaining_path: &[String],
    ) -> Option<TypePathResolution> {
        use kestrel_semantic_tree::ty::TyKind;

        if remaining_path.is_empty() {
            return None;
        }

        let segment = &remaining_path[0];

        // Get the bounds of the associated type (e.g., type Iter: Iterator)
        // Associated types use AssociatedTypeBoundsBehavior, not ConformancesBehavior
        let bounds = assoc_type.bounds()?;

        for bound in bounds.iter() {
            if let TyKind::Protocol { symbol: protocol, .. } = bound.kind() {
                let protocol_dyn = protocol.clone() as Arc<dyn Symbol<KestrelLanguage>>;

                // Look for the segment in this protocol
                for child in protocol_dyn.metadata().children() {
                    if child.metadata().kind() == KestrelSymbolKind::AssociatedType
                        && child.metadata().name().value == *segment
                    {
                        if let Some(symbol) = self.symbol_by_id(child.metadata().id()) {
                            if let Ok(inner_assoc_arc) = symbol.into_any_arc().downcast::<AssociatedTypeSymbol>() {
                                let span = container_ty.span().clone();

                                // If there are still more segments, recurse
                                if remaining_path.len() > 1 {
                                    let nested_container = Ty::qualified_associated_type(
                                        inner_assoc_arc.clone(),
                                        container_ty,
                                        span.clone(),
                                    );
                                    return self.resolve_nested_associated_type(
                                        &inner_assoc_arc,
                                        nested_container,
                                        &remaining_path[1..],
                                    );
                                }

                                let ty = Ty::qualified_associated_type(inner_assoc_arc, container_ty, span);
                                return Some(TypePathResolution::Resolved(ty));
                            }
                        }
                    }
                }

                // Check inherited protocols
                if let Some(SymbolResolution::Found(ids)) = self.find_in_inherited_protocols(&protocol_dyn, segment) {
                    if let Some(id) = ids.first() {
                        if let Some(symbol) = self.symbol_by_id(*id) {
                            if let Ok(inner_assoc_arc) = symbol.into_any_arc().downcast::<AssociatedTypeSymbol>() {
                                let span = container_ty.span().clone();
                                let ty = Ty::qualified_associated_type(inner_assoc_arc, container_ty, span);
                                return Some(TypePathResolution::Resolved(ty));
                            }
                        }
                    }
                }
            }
        }

        None
    }
}

impl Db for SemanticDatabase {
    fn symbol_by_id(&self, id: SymbolId) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        self.registry.get(id)
    }

    fn scope_for(&self, symbol_id: SymbolId) -> Arc<Scope> {
        // Check cache first
        if let Some(scope) = self.scope_cache.read().get(&symbol_id) {
            return scope.clone();
        }

        // Compute scope
        let scope = self.compute_scope(symbol_id);

        // Cache result
        self.scope_cache.write().insert(symbol_id, scope.clone());

        scope
    }

    fn resolve_name(&self, name: String, context: SymbolId) -> SymbolResolution {
        let mut current = Some(context);

        while let Some(id) = current {
            let scope = self.scope_for(id);

            // Check imports first
            if let Some(imported) = scope.imports.get(&name) {
                return if imported.len() == 1 {
                    SymbolResolution::Found(imported.clone())
                } else {
                    SymbolResolution::Ambiguous(imported.clone())
                };
            }

            // Check declarations
            if let Some(declared) = scope.declarations.get(&name) {
                return if declared.len() == 1 {
                    SymbolResolution::Found(declared.clone())
                } else {
                    SymbolResolution::Ambiguous(declared.clone())
                };
            }

            // Check inherited associated types from parent protocols
            // (conformances are resolved after scope computation, so we check at lookup time)
            if let Some(symbol) = self.symbol_by_id(id) {
                if symbol.metadata().kind() == KestrelSymbolKind::Protocol {
                    if let Some(result) = self.find_in_inherited_protocols(&symbol, &name) {
                        return result;
                    }
                }
            }

            current = scope.parent;
        }

        SymbolResolution::NotFound
    }

    fn imports_in_scope(&self, symbol_id: SymbolId) -> Vec<Arc<Import>> {
        let symbol = self.symbol_by_id(symbol_id).expect("symbol must exist");

        symbol
            .metadata()
            .children()
            .into_iter()
            .filter(|c| matches!(c.metadata().kind(), KestrelSymbolKind::Import))
            .filter_map(|import_symbol| {
                get_import_data(&import_symbol).map(|data| {
                    Arc::new(Import {
                        module_path: data.module_path().to_vec(),
                        alias: data.alias().map(|s| s.to_string()),
                        items: data
                            .items()
                            .iter()
                            .map(|i| ImportItem {
                                name: i.name.clone(),
                                alias: i.alias.clone(),
                                target_id: i.target_id,
                            })
                            .collect(),
                    })
                })
            })
            .collect()
    }

    fn is_visible_from(&self, target: SymbolId, context: SymbolId) -> bool {
        let target_symbol = self.symbol_by_id(target).expect("target symbol must exist");
        let context_symbol = self.symbol_by_id(context).expect("context symbol must exist");

        let checker = VisibilityChecker::new(&context_symbol);
        checker.is_visible(&target_symbol)
    }

    fn resolve_module_path(
        &self,
        path: Vec<String>,
        _context: SymbolId,
    ) -> Result<SymbolId, ModuleNotFoundError> {
        if path.is_empty() {
            return Err(ModuleNotFoundError {
                path: vec![],
                failed_segment_index: 0,
                path_span: 0..0,
                failed_segment_span: 0..0,
            });
        }

        // Find first segment using O(1) index lookup
        let first_segment = &path[0];
        let modules = self
            .registry
            .find_by_kind_and_name(KestrelSymbolKind::Module, first_segment);

        let mut current = match modules.into_iter().next() {
            Some(s) => s,
            None => {
                return Err(ModuleNotFoundError {
                    path: path.clone(),
                    failed_segment_index: 0,
                    path_span: 0..0,
                    failed_segment_span: 0..0,
                });
            }
        };

        // Resolve remaining segments
        for (index, segment) in path.iter().enumerate().skip(1) {
            let found = current
                .metadata()
                .visible_children()
                .into_iter()
                .find(|child| child.metadata().name().value == *segment);

            match found {
                Some(child) => current = child,
                None => {
                    return Err(ModuleNotFoundError {
                        path: path.clone(),
                        failed_segment_index: index,
                        path_span: 0..0,
                        failed_segment_span: 0..0,
                    });
                }
            }
        }

        Ok(current.metadata().id())
    }

    fn resolve_type_path(&self, path: Vec<String>, context: SymbolId) -> TypePathResolution {
        if path.is_empty() {
            return TypePathResolution::NotFound {
                segment: String::new(),
                index: 0,
            };
        }

        // Handle built-in primitive types
        if path.len() == 1 {
            let segment = &path[0];
            if let Some(ty) = resolve_primitive_type(segment, 0..0) {
                return TypePathResolution::Resolved(ty);
            }
        }

        let context_symbol = match self.symbol_by_id(context) {
            Some(s) => s,
            None => {
                return TypePathResolution::NotFound {
                    segment: path[0].clone(),
                    index: 0,
                };
            }
        };

        // First segment: use scope-aware name resolution
        let first = &path[0];
        let first_resolution = self.resolve_name(first.clone(), context);

        let mut current_symbol = match first_resolution {
            SymbolResolution::Found(ids) if ids.len() == 1 => match self.symbol_by_id(ids[0]) {
                Some(s) => s,
                None => {
                    return TypePathResolution::NotFound {
                        segment: first.clone(),
                        index: 0,
                    };
                }
            },
            SymbolResolution::Found(ids) => {
                return TypePathResolution::Ambiguous {
                    segment: first.clone(),
                    index: 0,
                    candidates: ids,
                };
            }
            SymbolResolution::Ambiguous(ids) => {
                return TypePathResolution::Ambiguous {
                    segment: first.clone(),
                    index: 0,
                    candidates: ids,
                };
            }
            SymbolResolution::NotFound => {
                return TypePathResolution::NotFound {
                    segment: first.clone(),
                    index: 0,
                };
            }
        };

        // Subsequent segments: search visible children
        let checker = VisibilityChecker::new(&context_symbol);
        for (index, segment) in path.iter().enumerate().skip(1) {
            // Special case: if current symbol is a TypeParameter, look up associated types
            // from its protocol bounds (e.g., T.Item where T: Iterator)
            if current_symbol.metadata().kind() == KestrelSymbolKind::TypeParameter {
                if let Some(type_param) = self.symbol_by_id(current_symbol.metadata().id()) {
                    if let Ok(type_param_arc) = type_param.clone().into_any_arc().downcast::<TypeParameterSymbol>() {
                        // Use context (the function/struct where this type is being resolved)
                        // instead of type_param's parent, since the parent may not be set correctly
                        if let Some(result) = self.resolve_associated_type_from_type_param_with_context(
                            &type_param_arc,
                            segment,
                            &path[index..],
                            index,
                            context,
                        ) {
                            return result;
                        }
                    }
                }
            }

            let matches = checker.find_visible_children(&current_symbol, segment);

            match matches.len() {
                0 => {
                    return TypePathResolution::NotFound {
                        segment: segment.clone(),
                        index,
                    };
                }
                1 => {
                    current_symbol = matches.into_iter().next().unwrap();
                }
                _ => {
                    return TypePathResolution::Ambiguous {
                        segment: segment.clone(),
                        index,
                        candidates: matches.iter().map(|s| s.metadata().id()).collect(),
                    };
                }
            }
        }

        // Handle TypeParameterSymbol specially
        if current_symbol.metadata().kind() == KestrelSymbolKind::TypeParameter {
            if let Some(symbol) = self.symbol_by_id(current_symbol.metadata().id()) {
                if let Ok(type_param_arc) = symbol.into_any_arc().downcast::<TypeParameterSymbol>() {
                    let span = type_param_arc.metadata().span().clone();
                    let ty = Ty::type_parameter(type_param_arc, span);
                    return TypePathResolution::Resolved(ty);
                }
            }
        }

        // Handle AssociatedTypeSymbol specially
        if current_symbol.metadata().kind() == KestrelSymbolKind::AssociatedType {
            if let Some(symbol) = self.symbol_by_id(current_symbol.metadata().id()) {
                if let Ok(assoc_type_arc) = symbol.into_any_arc().downcast::<AssociatedTypeSymbol>() {
                    let span = assoc_type_arc.metadata().span().clone();
                    let ty = Ty::associated_type(assoc_type_arc, span);
                    return TypePathResolution::Resolved(ty);
                }
            }
        }

        // Extract type from TypedBehavior
        let behaviors = current_symbol.metadata().behaviors();
        let typed_behaviors: Vec<_> = behaviors
            .iter()
            .filter_map(|b| {
                if matches!(b.kind(), KestrelBehaviorKind::Typed) {
                    b.as_ref().downcast_ref::<TypedBehavior>()
                } else {
                    None
                }
            })
            .collect();

        let type_alias_behavior = typed_behaviors
            .iter()
            .find(|tb| tb.ty().is_type_alias())
            .copied();

        let typed_behavior = type_alias_behavior.or_else(|| typed_behaviors.first().copied());

        match typed_behavior {
            Some(tb) => TypePathResolution::Resolved(tb.ty().clone()),
            None => TypePathResolution::NotAType {
                symbol_id: current_symbol.metadata().id(),
            },
        }
    }

    fn resolve_value_path(&self, path: Vec<String>, context: SymbolId) -> ValuePathResolution {
        if path.is_empty() {
            return ValuePathResolution::NotFound {
                segment: String::new(),
                index: 0,
            };
        }

        let context_symbol = match self.symbol_by_id(context) {
            Some(s) => s,
            None => {
                return ValuePathResolution::NotFound {
                    segment: path[0].clone(),
                    index: 0,
                };
            }
        };

        // First segment: use scope-aware name resolution
        let first = &path[0];
        let first_resolution = self.resolve_name(first.clone(), context);

        let first_symbols: Vec<_> = match first_resolution {
            SymbolResolution::Found(ids) => ids
                .iter()
                .filter_map(|id| self.symbol_by_id(*id))
                .collect(),
            SymbolResolution::Ambiguous(ids) => {
                let symbols: Vec<_> = ids
                    .iter()
                    .filter_map(|id| self.symbol_by_id(*id))
                    .collect();

                let all_functions = symbols
                    .iter()
                    .all(|s| s.metadata().kind() == KestrelSymbolKind::Function);

                if !all_functions {
                    return ValuePathResolution::Ambiguous {
                        segment: first.clone(),
                        index: 0,
                        candidates: ids,
                    };
                }
                symbols
            }
            SymbolResolution::NotFound => {
                return ValuePathResolution::NotFound {
                    segment: first.clone(),
                    index: 0,
                };
            }
        };

        if first_symbols.is_empty() {
            return ValuePathResolution::NotFound {
                segment: first.clone(),
                index: 0,
            };
        }

        // Single-segment paths
        if path.len() == 1 {
            return self.extract_value_from_symbols(&first_symbols, first, 0);
        }

        // Multi-segment paths require single resolution
        if first_symbols.len() > 1 {
            return ValuePathResolution::Ambiguous {
                segment: first.clone(),
                index: 0,
                candidates: first_symbols.iter().map(|s| s.metadata().id()).collect(),
            };
        }

        let mut current_symbol = first_symbols.into_iter().next().unwrap();
        let checker = VisibilityChecker::new(&context_symbol);

        for (index, segment) in path.iter().enumerate().skip(1) {
            let matches = checker.find_visible_children(&current_symbol, segment);

            // Last segment: handle overloads
            if index == path.len() - 1 {
                return self.extract_value_from_symbols(&matches, segment, index);
            }

            // Intermediate segments must resolve to single symbol
            match matches.len() {
                0 => {
                    return ValuePathResolution::NotFound {
                        segment: segment.clone(),
                        index,
                    };
                }
                1 => {
                    current_symbol = matches.into_iter().next().unwrap();
                }
                _ => {
                    return ValuePathResolution::Ambiguous {
                        segment: segment.clone(),
                        index,
                        candidates: matches.iter().map(|s| s.metadata().id()).collect(),
                    };
                }
            }
        }

        ValuePathResolution::NotFound {
            segment: path.last().cloned().unwrap_or_default(),
            index: path.len().saturating_sub(1),
        }
    }

    fn visible_children_from(
        &self,
        parent: SymbolId,
        context: SymbolId,
    ) -> Vec<Arc<dyn Symbol<KestrelLanguage>>> {
        let parent_symbol = match self.symbol_by_id(parent) {
            Some(s) => s,
            None => return Vec::new(),
        };

        parent_symbol
            .metadata()
            .visible_children()
            .into_iter()
            .filter(|child| {
                let child_id = child.metadata().id();
                self.is_visible_from(child_id, context)
            })
            .collect()
    }

    fn find_child_by_name(
        &self,
        parent: SymbolId,
        name: &str,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        let parent_symbol = self.symbol_by_id(parent)?;

        parent_symbol
            .metadata()
            .visible_children()
            .into_iter()
            .find(|child| child.metadata().name().value == name)
    }
}
