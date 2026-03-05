use std::sync::Arc;

use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::valued::ValueBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
use kestrel_semantic_tree::symbol::function::Parameter;
use kestrel_semantic_tree::ty::{Substitutions, Ty};
use kestrel_span::Spanned;
use kestrel_syntax_tree::utils::{extract_identifier_from_name, find_child, get_node_span};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::resolution::type_resolver::{TypeSyntaxContext, resolve_type_from_ty_node};

/// Binder for enum case declarations
pub struct EnumCaseBinder;

impl DeclarationBinder for EnumCaseBinder {
    fn bind_signature(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        let symbol_id = symbol.metadata().id();
        let span = symbol.metadata().span().clone();
        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // 2. Resolve attributes
        let attributes_behavior = crate::binders::utils::attributes::resolve_attributes(
            syntax,
            &source,
            file_id,
            context.diagnostics,
        );
        symbol.metadata().add_behavior(attributes_behavior);

        // Extract doc comment
        if let Some(doc) = crate::binders::utils::doc_comment::extract_doc_comment(syntax) {
            symbol.metadata().add_behavior(doc);
        }

        // 3. Check if case has parameters (associated values)
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

    // Parse each EnumCaseParameter (label: Type or just Type)
    param_list
        .children()
        .filter(|child| child.kind() == SyntaxKind::EnumCaseParameter)
        .enumerate()
        .filter_map(|(index, param_node)| {
            resolve_enum_case_parameter(&param_node, source, file_id, context_id, ctx, index)
        })
        .collect()
}

fn resolve_enum_case_parameter(
    param_node: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: SymbolId,
    ctx: &mut BindingContext,
    index: usize,
) -> Option<Parameter> {
    // Find the type (Ty node) - required for all parameters
    let ty_node = param_node.children().find(|c| c.kind() == SyntaxKind::Ty)?;

    // Find the optional label (Name node) - only present for named parameters
    let name_node = param_node.children().find(|c| c.kind() == SyntaxKind::Name);

    // Resolve type
    let mut type_ctx =
        TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
    let resolved_ty = resolve_type_from_ty_node(&ty_node, &mut type_ctx);

    // Get type span for synthetic names
    let ty_span = get_node_span(&ty_node, file_id);

    // Extract label if present, otherwise generate synthetic name
    let (label, bind_name, bind_span) = if let Some(name) = name_node {
        // Named parameter: `label: Type`
        let label_text = extract_identifier_from_name(&name)?;
        let label_span = get_node_span(&name, file_id);
        (
            Some(Spanned::new(label_text.clone(), label_span.clone())),
            label_text,
            label_span,
        )
    } else {
        // Unnamed parameter: just `Type`
        // Use index-based synthetic name for bind_name, no label for call site
        let synthetic_name = format!("_{}", index);
        (None, synthetic_name, ty_span)
    };

    Some(Parameter {
        // Enum case parameters use default borrow mode
        access_mode: kestrel_semantic_tree::behavior::callable::ParameterAccessMode::Borrow,
        // Label is None for unnamed parameters (positional matching in patterns)
        label,
        bind_name: Spanned::new(bind_name, bind_span),
        ty: resolved_ty,
        // Enum case parameters don't support defaults
        has_default: false,
    })
}

fn get_parent_enum_type(symbol: &Arc<dyn Symbol<KestrelLanguage>>, span: kestrel_span::Span) -> Ty {
    // Get parent enum symbol
    if let Some(parent) = symbol.metadata().parent()
        && let Ok(enum_sym) = parent.downcast_arc::<EnumSymbol>()
    {
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
    Ty::error(span)
}
