//! Validates type alias declarations after binding.
//!
//! Checks that were previously in the type_alias binder are now here:
//! - Bounds only on protocol associated types (not struct/module type aliases)
//! - Type aliases require `= Type` (except abstract associated types in protocols)
//! - Associated type bounds must be protocols
//! - Qualified bindings reference valid protocol conformances
//! - Unqualified bindings are not ambiguous
//! - Bound types satisfy protocol constraints

use std::collections::HashSet;
use std::sync::Arc;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use kestrel_semantic_model::{ConformancesForSymbol, ConformsToQuery, ExtensionTargetFor};
use kestrel_semantic_tree::behavior::conforms_to::QualifiedBindingBehavior;
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::associated_type::{
    AssociatedTypeBoundsBehavior, AssociatedTypeSymbol,
};
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasTypedBehavior;
use kestrel_semantic_tree::ty::{Constraint, Ty, TyKind};
use semantic_tree::symbol::Symbol;

pub mod diagnostics;
use diagnostics::*;

/// Collected type alias info for validation in finalize
struct CollectedTypeAlias {
    symbol: Arc<dyn Symbol<KestrelLanguage>>,
    parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
    parent_kind: Option<KestrelSymbolKind>,
}

/// Collected associated type info for validation in finalize
struct CollectedAssociatedType {
    symbol: Arc<dyn Symbol<KestrelLanguage>>,
}

pub struct TypeAliasValidationAnalyzer {
    type_aliases: Vec<CollectedTypeAlias>,
    associated_types: Vec<CollectedAssociatedType>,
}

impl TypeAliasValidationAnalyzer {
    pub fn new() -> Self {
        Self {
            type_aliases: Vec::new(),
            associated_types: Vec::new(),
        }
    }
}

impl Default for TypeAliasValidationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for TypeAliasValidationAnalyzer {
    fn name(&self) -> &'static str {
        "type_alias_validation"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        _ctx: &mut AnalysisContext,
    ) {
        match symbol.metadata().kind() {
            KestrelSymbolKind::TypeAlias => {
                let parent = symbol.metadata().parent();
                let parent_kind = parent.as_ref().map(|p| p.metadata().kind());
                self.type_aliases.push(CollectedTypeAlias {
                    symbol: symbol.clone(),
                    parent,
                    parent_kind,
                });
            },
            KestrelSymbolKind::AssociatedType => {
                self.associated_types.push(CollectedAssociatedType {
                    symbol: symbol.clone(),
                });
            },
            _ => {},
        }
    }

    fn finalize(&mut self, ctx: &mut AnalysisContext) {
        // Validate associated type bounds are protocols
        for assoc in &self.associated_types {
            validate_bounds_are_protocols(&assoc.symbol, ctx);
        }

        // Validate type aliases
        for alias in &self.type_aliases {
            let context = determine_context(alias.parent_kind);

            // Check 1: Bounds on non-protocol type aliases are not allowed
            if context != AliasContext::Protocol
                && alias
                    .symbol
                    .metadata()
                    .get_behavior::<AssociatedTypeBoundsBehavior>()
                    .is_some()
            {
                ctx.report(AssociatedTypeBoundsInWrongContextError {
                    span: alias.symbol.metadata().span().clone(),
                    name: alias.symbol.metadata().name().value.clone(),
                });
            }

            // Check 2: Type aliases require `= Type` (except in protocols)
            if alias
                .symbol
                .metadata()
                .get_behavior::<TypeAliasTypedBehavior>()
                .is_none()
            {
                let diag_context = match context {
                    AliasContext::Module => Some(TypeAliasContext::ModuleLevel),
                    AliasContext::ConcreteType => {
                        Some(TypeAliasContext::ConcreteTypeWithoutConformance)
                    },
                    AliasContext::Extension => Some(TypeAliasContext::ExtensionWithoutConformance),
                    AliasContext::Protocol => None,
                };

                if let Some(diag_ctx) = diag_context {
                    ctx.report(TypeAliasRequiresTypeError {
                        span: alias.symbol.metadata().span().clone(),
                        name: alias.symbol.metadata().name().value.clone(),
                        context: diag_ctx,
                    });
                }
            }

            // Checks 4-7: Validation for struct/extension associated type bindings
            if (context == AliasContext::ConcreteType || context == AliasContext::Extension)
                && let Some(parent) = &alias.parent
            {
                let name = alias.symbol.metadata().name().value.clone();
                let span = alias.symbol.metadata().span().clone();

                // Get parent display name for error messages
                let parent_display_name =
                    if parent.metadata().kind() == KestrelSymbolKind::Extension {
                        ctx.model
                            .query(ExtensionTargetFor {
                                symbol_id: parent.metadata().id(),
                            })
                            .map(|ty| format!("{}", ty))
                            .unwrap_or_else(|| "(extension)".to_string())
                    } else {
                        parent.metadata().name().value.clone()
                    };

                // Use direct conformances (not including extension-provided ones) for
                // qualified/unqualified validation. Extension-provided conformances have
                // their own type alias bindings in the extension body.
                let conformances = ctx.model.query(ConformancesForSymbol {
                    symbol_id: parent.metadata().id(),
                });

                // Check qualified vs unqualified binding
                if let Some(qualified) = alias
                    .symbol
                    .metadata()
                    .get_behavior::<QualifiedBindingBehavior>()
                {
                    validate_qualified_binding(
                        &qualified,
                        &name,
                        &parent_display_name,
                        &span,
                        &conformances,
                        ctx,
                    );
                } else {
                    validate_unqualified_binding(&name, &span, &conformances, ctx);
                }

                // Check 7: Constraint satisfaction (only if we have a resolved type)
                if let Some(typed) = alias
                    .symbol
                    .metadata()
                    .get_behavior::<TypeAliasTypedBehavior>()
                {
                    validate_constraint_satisfaction(
                        typed.resolved_ty(),
                        &name,
                        parent,
                        &span,
                        ctx,
                    );
                }
            }
        }
    }
}

/// Context for a type alias declaration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AliasContext {
    Protocol,
    ConcreteType,
    Extension,
    Module,
}

fn determine_context(parent_kind: Option<KestrelSymbolKind>) -> AliasContext {
    match parent_kind {
        Some(KestrelSymbolKind::Protocol) => AliasContext::Protocol,
        Some(KestrelSymbolKind::Struct | KestrelSymbolKind::Enum) => AliasContext::ConcreteType,
        Some(KestrelSymbolKind::Extension) => AliasContext::Extension,
        _ => AliasContext::Module,
    }
}

/// Check 3: Validate that all bounds on an associated type are protocols
fn validate_bounds_are_protocols(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    ctx: &mut AnalysisContext,
) {
    let Some(bounds_beh) = symbol
        .metadata()
        .get_behavior::<AssociatedTypeBoundsBehavior>()
    else {
        return;
    };

    for bound in bounds_beh.bounds() {
        match bound.kind() {
            TyKind::Protocol { .. } | TyKind::Error => {},
            _ => {
                ctx.report(NotAProtocolBoundError {
                    span: symbol.metadata().span().clone(),
                    name: format!("{}", bound),
                });
            },
        }
    }
}

/// Checks 4-5: Validate a qualified binding (type Protocol.Item = T)
fn validate_qualified_binding(
    qualified: &QualifiedBindingBehavior,
    type_name: &str,
    parent_display_name: &str,
    span: &kestrel_span::Span,
    conformances: &[Ty],
    ctx: &mut AnalysisContext,
) {
    let protocol_name = qualified.protocol_name();

    // Check 4: Does the parent conform to this protocol?
    let conforms_to_protocol = conformances.iter().any(|conf| {
        if let TyKind::Protocol { symbol, .. } = conf.kind() {
            symbol.metadata().name().value == protocol_name
        } else {
            false
        }
    });

    if !conforms_to_protocol {
        ctx.report(QualifiedBindingNotConformingError {
            span: span.clone(),
            struct_name: parent_display_name.to_string(),
            protocol_name: protocol_name.to_string(),
        });
        return;
    }

    // Check 5: Does the protocol have this associated type?
    let protocol_has_type = conformances.iter().any(|conf| {
        if let TyKind::Protocol { symbol, .. } = conf.kind()
            && symbol.metadata().name().value == protocol_name
        {
            let protocol_dyn = symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
            return protocol_dyn.metadata().children().iter().any(|child| {
                child.metadata().kind() == KestrelSymbolKind::AssociatedType
                    && child.metadata().name().value == type_name
            });
        }
        false
    });

    if !protocol_has_type {
        ctx.report(QualifiedBindingWrongProtocolError {
            span: span.clone(),
            protocol_name: protocol_name.to_string(),
            type_name: type_name.to_string(),
        });
    }
}

/// Check 6: Validate an unqualified binding (type Item = T) is not ambiguous
fn validate_unqualified_binding(
    type_name: &str,
    span: &kestrel_span::Span,
    conformances: &[Ty],
    ctx: &mut AnalysisContext,
) {
    let protocols_with_type: Vec<String> = conformances
        .iter()
        .filter_map(|conf| {
            if let TyKind::Protocol { symbol, .. } = conf.kind() {
                let protocol_dyn = symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
                let has_type = protocol_dyn.metadata().children().iter().any(|child| {
                    child.metadata().kind() == KestrelSymbolKind::AssociatedType
                        && child.metadata().name().value == type_name
                });
                if has_type {
                    return Some(symbol.metadata().name().value.clone());
                }
            }
            None
        })
        .collect();

    if protocols_with_type.len() > 1 {
        ctx.report(AmbiguousAssociatedTypeError {
            span: span.clone(),
            type_name: type_name.to_string(),
            protocols: protocols_with_type,
        });
    }
}

/// Check 7: Validate that the bound type satisfies protocol constraints on the associated type
fn validate_constraint_satisfaction(
    bound_type: &Ty,
    type_name: &str,
    parent: &Arc<dyn Symbol<KestrelLanguage>>,
    span: &kestrel_span::Span,
    ctx: &mut AnalysisContext,
) {
    let conformances = ctx.model.query(ConformancesForSymbol {
        symbol_id: parent.metadata().id(),
    });

    let all_protocols = collect_all_inherited_protocols(&conformances, ctx.model);

    // Check direct bounds on the associated type declaration
    for protocol in &all_protocols {
        if let TyKind::Protocol {
            symbol: protocol_symbol,
            ..
        } = protocol.kind()
        {
            let protocol_dyn = protocol_symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>;

            for child in protocol_dyn.metadata().children() {
                if child.metadata().kind() == KestrelSymbolKind::AssociatedType
                    && child.metadata().name().value == type_name
                {
                    if let Ok(assoc_type_symbol) = child.downcast_arc::<AssociatedTypeSymbol>()
                        && let Some(bounds) = assoc_type_symbol.bounds()
                    {
                        validate_type_satisfies_bounds(
                            bound_type,
                            &bounds,
                            type_name,
                            span.clone(),
                            ctx,
                        );
                    }
                }
            }
        }
    }

    // Check inherited where clause constraints
    validate_inherited_where_clause_constraints(type_name, bound_type, &conformances, span, ctx);
}

/// Check if a type satisfies a list of protocol bounds
fn validate_type_satisfies_bounds(
    bound_type: &Ty,
    required_bounds: &[Ty],
    type_name: &str,
    span: kestrel_span::Span,
    ctx: &mut AnalysisContext,
) {
    let bound_type_name = format!("{}", bound_type);

    for required_protocol in required_bounds {
        if matches!(required_protocol.kind(), TyKind::Error) {
            continue;
        }

        if let TyKind::Protocol {
            symbol: required_proto_symbol,
            ..
        } = required_protocol.kind()
        {
            let required_protocol_name = required_proto_symbol.metadata().name().value.clone();

            // Type parameters, Self, and Error types are assumed to satisfy bounds
            let conforms = match bound_type.kind() {
                TyKind::TypeParameter(_) | TyKind::SelfType | TyKind::Error => true,
                _ => ctx.model.query(ConformsToQuery::new(
                    bound_type,
                    required_proto_symbol.metadata().id(),
                )),
            };

            if !conforms {
                ctx.report(AssociatedTypeConstraintNotSatisfiedError {
                    span,
                    type_name: type_name.to_string(),
                    bound_type: bound_type_name.clone(),
                    required_protocol: required_protocol_name,
                });
                return; // Only report the first violation
            }
        }
    }
}

/// Validate inherited where clause constraints on associated types
fn validate_inherited_where_clause_constraints(
    type_name: &str,
    bound_type: &Ty,
    conformances: &[Ty],
    binding_span: &kestrel_span::Span,
    ctx: &mut AnalysisContext,
) {
    let all_protocols = collect_all_inherited_protocols(conformances, ctx.model);

    for protocol in &all_protocols {
        if let TyKind::Protocol {
            symbol: protocol_symbol,
            ..
        } = protocol.kind()
        {
            let Some(generics) = protocol_symbol
                .metadata()
                .get_behavior::<GenericsBehavior>()
            else {
                continue;
            };

            for constraint in &generics.where_clause().constraints {
                match constraint {
                    Constraint::TypeBound {
                        param_name, bounds, ..
                    } => {
                        if let Some(assoc_name) = param_name.split('.').next_back()
                            && assoc_name == type_name
                        {
                            validate_type_satisfies_bounds(
                                bound_type,
                                bounds,
                                type_name,
                                binding_span.clone(),
                                ctx,
                            );
                        }
                    },
                    Constraint::InheritedAssociatedTypeBound { path, bounds, .. } => {
                        if let Some(assoc_name) = path.split('.').next_back()
                            && assoc_name == type_name
                        {
                            validate_type_satisfies_bounds(
                                bound_type,
                                bounds,
                                type_name,
                                binding_span.clone(),
                                ctx,
                            );
                        }
                    },
                    Constraint::TypeEquality { .. }
                    | Constraint::NegativeBound { .. }
                    | Constraint::SelfBound { .. } => {},
                }
            }
        }
    }
}

/// Collect all protocols from conformances, including inherited ones, with cycle detection
fn collect_all_inherited_protocols(
    conformances: &[Ty],
    model: &kestrel_semantic_model::SemanticModel,
) -> Vec<Ty> {
    let mut all_protocols = Vec::new();
    let mut to_check: Vec<_> = conformances.to_vec();
    let mut visited = HashSet::new();
    while let Some(conformance) = to_check.pop() {
        if let TyKind::Protocol { symbol, .. } = conformance.kind() {
            let id = symbol.metadata().id();
            if !visited.insert(id) {
                continue;
            }
            all_protocols.push(conformance.clone());
            let inherited = model.query(ConformancesForSymbol { symbol_id: id });
            to_check.extend(inherited);
        }
    }
    all_protocols
}
