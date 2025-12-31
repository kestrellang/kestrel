use std::sync::Arc;

use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::valued::ValueBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
use kestrel_semantic_tree::symbol::function::Parameter;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::{Substitutions, Ty};
use kestrel_span::Spanned;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use kestrel_syntax_tree::utils::{extract_identifier_from_name, find_child, get_node_span};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::resolution::type_resolver::{resolve_type_from_ty_node, TypeSyntaxContext};

/// Binder for enum case declarations
pub struct EnumCaseBinder;

impl DeclarationBinder for EnumCaseBinder {
    fn bind_signature(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // 1. Guard: Only process enum case symbols
        if symbol.metadata().kind() != KestrelSymbolKind::EnumCase {
            return;
        }

        let symbol_id = symbol.metadata().id();
        let span = symbol.metadata().span().clone();
        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // 2. Check if case has parameters (associated values)
        // Look for EnumCaseParameterList in the syntax
        let has_parameters = syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::EnumCaseParameterList);

        if !has_parameters {
            // Simple case without associated values - add ValueBehavior with enum type
            let enum_type = get_parent_enum_type(symbol, span.clone());
            let value_behavior = ValueBehavior::new(enum_type, span);
            symbol.metadata().add_behavior(value_behavior);
            return;
        }

        // 3. Resolve parameters
        let resolved_params =
            resolve_enum_case_parameters(syntax, &source, file_id, symbol_id, context);

        // 4. Check for empty parameters - treat as no CallableBehavior
        if resolved_params.is_empty() {
            return;
        }

        // 5. Get the parent enum's type for the "return type" of the case constructor
        let return_type = get_parent_enum_type(symbol, span.clone());

        // 6. Create and attach CallableBehavior
        // Enum cases don't have a receiver (they're like static constructors)
        let callable = CallableBehavior::new(resolved_params, return_type, span);
        symbol.metadata().add_behavior(callable);
    }
}

fn resolve_enum_case_parameters(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: SymbolId,
    ctx: &mut BindingContext,
) -> Vec<Parameter> {
    // Find EnumCaseParameterList
    let param_list = match find_child(syntax, SyntaxKind::EnumCaseParameterList) {
        Some(node) => node,
        None => return Vec::new(),
    };

    // Parse each EnumCaseParameter (label: Type)
    param_list
        .children()
        .filter(|child| child.kind() == SyntaxKind::EnumCaseParameter)
        .filter_map(|param_node| {
            resolve_enum_case_parameter(&param_node, source, file_id, context_id, ctx)
        })
        .collect()
}

fn resolve_enum_case_parameter(
    param_node: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: SymbolId,
    ctx: &mut BindingContext,
) -> Option<Parameter> {
    // Find the label (Name node) and type (Ty node)
    let name_node = param_node
        .children()
        .find(|c| c.kind() == SyntaxKind::Name)?;
    let ty_node = param_node.children().find(|c| c.kind() == SyntaxKind::Ty)?;

    // Extract label text
    let label_text = extract_identifier_from_name(&name_node)?;
    let label_span = get_node_span(&name_node, file_id);

    // Resolve type
    let mut type_ctx =
        TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
    let resolved_ty = resolve_type_from_ty_node(&ty_node, &mut type_ctx);

    Some(Parameter {
        // Enum case parameters use default borrow mode
        access_mode: kestrel_semantic_tree::behavior::callable::ParameterAccessMode::Borrow,
        // Enum case parameters always have labels (like init parameters)
        label: Some(Spanned::new(label_text.clone(), label_span.clone())),
        bind_name: Spanned::new(label_text, label_span),
        ty: resolved_ty,
    })
}

fn get_parent_enum_type(symbol: &Arc<dyn Symbol<KestrelLanguage>>, span: kestrel_span::Span) -> Ty {
    // Get parent enum symbol
    if let Some(parent) = symbol.metadata().parent() {
        if let Ok(enum_sym) = parent.downcast_arc::<EnumSymbol>() {
            let type_params = enum_sym.type_parameters();
            if type_params.is_empty() {
                // Non-generic enum - return simple enum type
                return Ty::r#enum(enum_sym, span);
            } else {
                // Generic enum - create type with type parameter references
                // e.g., for Option[T], return Option[T] where T maps to TypeParameter(T)
                // This allows type inference to properly unify when called
                let mut substitutions = Substitutions::new();
                for param in &type_params {
                    let param_ty = Ty::type_parameter(param.clone(), span.clone());
                    substitutions.insert(param.metadata().id(), param_ty);
                }
                return Ty::generic_enum(enum_sym, substitutions, span);
            }
        }
    }
    Ty::error(span)
}
