use std::sync::Arc;

use kestrel_semantic_tree::behavior::attributes::AttributesBehavior;
use kestrel_semantic_tree::behavior::conforms_to::{ConformsToBehavior, QualifiedBindingBehavior};
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeBoundsBehavior;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasTypedBehavior;
use kestrel_semantic_tree::ty::TyKind;
use kestrel_syntax_tree::{SyntaxElement, SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::resolution::type_resolver::{TypeSyntaxContext, resolve_type_from_ty_node};
use kestrel_syntax_tree::utils::find_child;

/// Binder for type alias declarations.
///
/// Purely handles resolution: resolving types, bounds, generics, and attaching
/// behaviors. All validation (bounds context, missing types, conformance checks,
/// constraint satisfaction) is handled by TypeAliasValidationAnalyzer.
pub struct TypeAliasBinder;

impl DeclarationBinder for TypeAliasBinder {
    fn bind_signature(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        let symbol_kind = symbol.metadata().kind();
        let symbol_id = symbol.metadata().id();

        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // Extract doc comment
        if let Some(doc) = crate::binders::utils::doc_comment::extract_doc_comment(syntax) {
            symbol.metadata().add_behavior(doc);
        }

        match symbol_kind {
            KestrelSymbolKind::AssociatedType => {
                // Associated type in protocol: resolve bounds and optional default
                bind_associated_type(symbol, syntax, &source, file_id, context);
            },
            KestrelSymbolKind::TypeAlias => {
                // Regular type alias or associated type binding: resolve aliased type
                // Enter cycle detector
                if semantic_tree::cycle::CycleDetector::enter_ref(
                    context.type_alias_cycle_detector,
                    symbol_id,
                )
                .is_err()
                {
                    return;
                }

                let alias_context = determine_context(symbol.metadata().parent().as_ref());
                let name = symbol.metadata().name().value.clone();

                // Resolve attributes for module-level type aliases
                if alias_context == TypeAliasContext::Module {
                    let attributes_behavior = crate::binders::utils::attributes::resolve_attributes(
                        syntax,
                        &source,
                        file_id,
                        context.diagnostics,
                    );
                    symbol.metadata().add_behavior(attributes_behavior.clone());

                    // Process @builtin attribute if present
                    process_builtin_attribute(symbol, &attributes_behavior, &source, context);
                }

                // Resolve bounds if present (even for non-protocol context — the analyzer validates context)
                let bounds = resolve_associated_type_bounds(syntax, &source, file_id, symbol_id, context);
                if !bounds.is_empty() {
                    let bounds_behavior = AssociatedTypeBoundsBehavior::new(bounds);
                    symbol.metadata().add_behavior(bounds_behavior);
                }

                // Attach QualifiedBindingBehavior if this is a qualified binding (type Protocol.Item = T)
                if let Some(qualified_name) = extract_qualified_protocol_name(syntax) {
                    symbol.metadata().add_behavior(QualifiedBindingBehavior::new(qualified_name));
                }

                // Extract type parameters and resolve where clause bounds
                let generics_behavior = crate::binders::utils::generics::resolve_generics(
                    syntax, &source, file_id, symbol_id, context,
                );
                symbol.metadata().add_behavior(generics_behavior);

                // Extract and resolve the aliased type from syntax
                if let Some(resolved_type) =
                    resolve_aliased_type_from_syntax(syntax, &source, file_id, symbol_id, context)
                {
                    // Add ConformsToBehavior for struct/extension associated type bindings
                    if (alias_context == TypeAliasContext::ConcreteType
                        || alias_context == TypeAliasContext::Extension)
                        && let Some(parent) = symbol.metadata().parent()
                    {
                        let qualified_protocol_name = extract_qualified_protocol_name(syntax);
                        add_conforms_to_behavior(
                            symbol,
                            &name,
                            &parent,
                            qualified_protocol_name.as_deref(),
                            context,
                        );
                    }

                    let type_alias_typed_behavior = TypeAliasTypedBehavior::new(resolved_type);
                    symbol.metadata().add_behavior(type_alias_typed_behavior);
                }

                // Exit cycle detector
                semantic_tree::cycle::CycleDetector::exit_ref(context.type_alias_cycle_detector);
            },
            _ => {},
        }
    }
}

/// Determines the context in which a type alias declaration appears
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeAliasContext {
    /// In a protocol body - creates AssociatedTypeSymbol
    Protocol,
    /// In a struct or enum body - creates TypeAliasSymbol (associated type binding)
    ConcreteType,
    /// In an extension body - creates TypeAliasSymbol (associated type binding)
    Extension,
    /// At module/file level - creates regular TypeAliasSymbol
    Module,
}

/// Determine context based on parent symbol kind
fn determine_context(parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>) -> TypeAliasContext {
    match parent {
        Some(p) => match p.metadata().kind() {
            KestrelSymbolKind::Protocol => TypeAliasContext::Protocol,
            KestrelSymbolKind::Struct | KestrelSymbolKind::Enum => TypeAliasContext::ConcreteType,
            KestrelSymbolKind::Extension => TypeAliasContext::Extension,
            _ => TypeAliasContext::Module,
        },
        None => TypeAliasContext::Module,
    }
}

/// Bind an associated type symbol (resolve bounds and default)
fn bind_associated_type(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context: &mut BindingContext,
) {
    let symbol_id = symbol.metadata().id();

    // Resolve bounds if present (: Equatable, Hashable)
    let bounds = resolve_associated_type_bounds(syntax, source, file_id, symbol_id, context);
    if !bounds.is_empty() {
        let bounds_behavior = AssociatedTypeBoundsBehavior::new(bounds);
        symbol.metadata().add_behavior(bounds_behavior);
    }

    // Resolve where clause if present (where Iter.Item = Item)
    let generics = crate::binders::utils::generics::resolve_generics(
        syntax, source, file_id, symbol_id, context,
    );
    if !generics.where_clause().constraints.is_empty() {
        symbol.metadata().add_behavior(generics);
    }

    // Resolve default type if present (= Int)
    if let Some(default_type) =
        resolve_aliased_type_from_syntax(syntax, source, file_id, symbol_id, context)
    {
        let typed_behavior = TypedBehavior::new(default_type, symbol.metadata().span().clone());
        symbol.metadata().add_behavior(typed_behavior);
    }
}

/// Resolve associated type bounds from syntax.
///
/// Resolves all types after the colon and before equals. Does not validate
/// that they are protocols — that is done by the TypeAliasValidationAnalyzer.
fn resolve_associated_type_bounds(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
) -> Vec<kestrel_semantic_tree::ty::Ty> {
    let mut bounds = Vec::new();

    let mut after_colon = false;
    for child in syntax.children_with_tokens() {
        match &child {
            SyntaxElement::Token(tok) if tok.kind() == SyntaxKind::Colon => {
                after_colon = true;
            },
            SyntaxElement::Token(tok) if tok.kind() == SyntaxKind::Equals => {
                break;
            },
            SyntaxElement::Node(node)
                if after_colon
                    && (node.kind() == SyntaxKind::Ty || node.kind() == SyntaxKind::TyPath) =>
            {
                let mut type_ctx =
                    TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
                let bound_ty = resolve_type_from_ty_node(node, &mut type_ctx);
                bounds.push(bound_ty);
            },
            _ => {},
        }
    }

    bounds
}

/// Resolve the aliased type from a TypeAliasDeclaration syntax node.
///
/// Returns None if no aliased type is present (e.g., for abstract associated types in protocols).
fn resolve_aliased_type_from_syntax(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
) -> Option<kestrel_semantic_tree::ty::Ty> {
    let aliased_type_node = find_child(syntax, SyntaxKind::AliasedType)?;

    let ty_node =
        find_child(&aliased_type_node, SyntaxKind::Ty).unwrap_or_else(|| aliased_type_node.clone());

    let mut type_ctx =
        TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
    Some(resolve_type_from_ty_node(&ty_node, &mut type_ctx))
}

/// Extract the first identifier token from a Path node.
fn first_identifier_from_path(path_node: &SyntaxNode) -> Option<String> {
    path_node
        .children()
        .find(|c| c.kind() == SyntaxKind::PathElement)
        .and_then(|elem| {
            elem.children_with_tokens()
                .find_map(|e| e.into_token().filter(|t| t.kind() == SyntaxKind::Identifier))
                .map(|t| t.text().to_string())
        })
}

/// Extract the first path segment name from a Ty node.
fn extract_path_name_from_ty_node(ty_node: &SyntaxNode) -> Option<String> {
    if let Some(path) = find_child(ty_node, SyntaxKind::Path) {
        return first_identifier_from_path(&path);
    }
    if let Some(ty_path) = find_child(ty_node, SyntaxKind::TyPath) {
        if let Some(path) = find_child(&ty_path, SyntaxKind::Path) {
            return first_identifier_from_path(&path);
        }
    }
    if ty_node.kind() == SyntaxKind::TyPath {
        if let Some(path) = find_child(ty_node, SyntaxKind::Path) {
            return first_identifier_from_path(&path);
        }
    }
    None
}

/// Extract the qualified protocol name from a type alias syntax node.
///
/// For qualified bindings like `type Addable.Output = Int64`, returns `Some("Addable")`.
/// For unqualified bindings like `type Output = Int64`, returns `None`.
fn extract_qualified_protocol_name(syntax: &SyntaxNode) -> Option<String> {
    let target_node = find_child(syntax, SyntaxKind::AssociatedTypeTarget)?;
    let ty_node = find_child(&target_node, SyntaxKind::Ty)?;
    extract_path_name_from_ty_node(&ty_node)
}

/// Add ConformsToBehavior to a type alias that binds an associated type.
///
/// Finds the protocol(s) that define the associated type and creates a
/// ConformsToBehavior for each one, allowing witness generation to properly
/// bind associated types.
fn add_conforms_to_behavior(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    type_name: &str,
    parent: &Arc<dyn Symbol<KestrelLanguage>>,
    qualified_protocol_name: Option<&str>,
    ctx: &mut BindingContext,
) {
    let conformances = ctx.model.query(kestrel_semantic_model::ConformancesForSymbol {
        symbol_id: parent.metadata().id(),
    });

    let all_protocol_tys = collect_all_inherited_protocols(&conformances, ctx.model);

    for protocol_ty in all_protocol_tys {
        let protocol_symbol = match protocol_ty.kind() {
            TyKind::Protocol { symbol, .. } => symbol.clone(),
            _ => continue,
        };
        // For qualified bindings, only add for the specified protocol
        if let Some(qualified_name) = qualified_protocol_name
            && protocol_symbol.metadata().name().value != qualified_name
        {
            continue;
        }

        let protocol_dyn = protocol_symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>;

        for child in protocol_dyn.metadata().children() {
            if child.metadata().kind() == KestrelSymbolKind::AssociatedType
                && child.metadata().name().value == type_name
            {
                let conforms_to = ConformsToBehavior::new(
                    protocol_symbol.clone(),
                    type_name.to_string(),
                    Some(child.metadata().id()),
                );
                symbol.metadata().add_behavior(conforms_to);
                break;
            }
        }
    }
}

/// Collect all protocols from conformances, including inherited protocols,
/// with cycle detection to prevent infinite loops.
pub(crate) fn collect_all_inherited_protocols(
    conformances: &[kestrel_semantic_tree::ty::Ty],
    model: &kestrel_semantic_model::SemanticModel,
) -> Vec<kestrel_semantic_tree::ty::Ty> {
    let mut all_protocols = Vec::new();
    let mut to_check: Vec<_> = conformances.to_vec();
    let mut visited = std::collections::HashSet::new();
    while let Some(conformance) = to_check.pop() {
        if let TyKind::Protocol { symbol, .. } = conformance.kind() {
            let id = symbol.metadata().id();
            if !visited.insert(id) {
                continue;
            }
            all_protocols.push(conformance.clone());
            let inherited = model.query(kestrel_semantic_model::ConformancesForSymbol {
                symbol_id: id,
            });
            to_check.extend(inherited);
        }
    }
    all_protocols
}

fn process_builtin_attribute(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    attributes: &AttributesBehavior,
    source: &str,
    context: &mut BindingContext,
) {
    let registry = context.model.builtin_registry().clone();
    crate::binders::utils::attributes::validate_builtin_attribute(
        symbol, attributes, source, context,
        "type alias",
        |k| k.is_type_alias(),
        |f| registry.type_alias(f),
    );
}
