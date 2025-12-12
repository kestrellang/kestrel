use std::sync::Arc;

use kestrel_semantic_tree::behavior::member_access::MemberAccessBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::field::FieldSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::{Span, Spanned};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::resolution::type_resolver::{TypeSyntaxContext, resolve_type_from_ty_node};
use crate::declaration_binder::{BindingContext, DeclarationBinder};
use kestrel_semantic_tree::behavior::visibility::{Visibility, find_visibility_scope};
use kestrel_syntax_tree::utils::{
    extract_name, extract_visibility, find_child, get_node_span, get_visibility_span,
};

/// Binder for field declarations
pub struct FieldBinder;

impl DeclarationBinder for FieldBinder {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        // Extract name
        let name_str = extract_name(syntax)?;
        let name_node = find_child(syntax, SyntaxKind::Name)?;
        let name_span = get_node_span(&name_node, source);

        // Get full span
        let full_span = get_node_span(syntax, source);

        // Extract visibility
        let visibility_str = extract_visibility(syntax);
        let visibility_enum = visibility_str.as_deref().and_then(Visibility::from_keyword);

        let visibility_span = get_visibility_span(syntax, source).unwrap_or(name_span.clone());

        // Determine visibility scope
        let visibility_scope = find_visibility_scope(visibility_enum.as_ref(), parent, root);

        // Create visibility behavior
        let visibility_behavior =
            VisibilityBehavior::new(visibility_enum, visibility_span, visibility_scope);

        // Check if this field is static
        let is_static = syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::StaticModifier);

        // Check if this field is mutable (var vs let)
        let is_mutable = syntax
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .any(|tok| tok.kind() == SyntaxKind::Var);

        // Use error type as placeholder - actual type will be resolved in bind phase from syntax
        let field_type = Ty::error(full_span.clone());

        // Create the name object
        let name = Spanned::new(name_str, name_span);

        // Create the field symbol
        let field_symbol = FieldSymbol::new(
            name,
            full_span,
            visibility_behavior,
            is_static,
            is_mutable,
            field_type,
            parent.cloned(),
        );
        let field_arc = Arc::new(field_symbol);

        let field_arc_dyn = field_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

        // Add to parent if exists
        if let Some(parent) = parent {
            parent.metadata().add_child(&field_arc_dyn);
        }

        Some(field_arc)
    }

    fn bind_declaration(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // Only process field symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Field {
            return;
        }

        let symbol_id = symbol.metadata().id();
        let span = symbol.metadata().span().clone();

        let source = context.source_for_symbol(symbol);

        // Resolve the type directly from syntax
        let resolved_type = resolve_field_type_from_syntax(syntax, &source, symbol_id, context);

        // Add a TypedBehavior with the resolved type
        let typed_behavior = TypedBehavior::new(resolved_type.clone(), span);
        symbol.metadata().add_behavior(typed_behavior);

        // Add a MemberAccessBehavior so this field can be accessed via dot notation
        let field_name = symbol.metadata().name().value.clone();

        // Get mutability from the FieldSymbol
        let is_mutable = symbol
            .as_ref()
            .downcast_ref::<FieldSymbol>()
            .map(|f| f.is_mutable())
            .unwrap_or(false);

        let member_access_behavior =
            MemberAccessBehavior::new(field_name, resolved_type, is_mutable);
        symbol.metadata().add_behavior(member_access_behavior);
    }
}

/// Resolve the field type from a FieldDeclaration syntax node
/// This extracts the type from syntax and immediately resolves it
fn resolve_field_type_from_syntax(
    syntax: &SyntaxNode,
    source: &str,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
) -> Ty {
    // Find the Ty node and resolve using shared utility
    if let Some(ty_node) = syntax
        .children()
        .find(|child| child.kind() == SyntaxKind::Ty)
    {
        let mut type_ctx = TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, context_id);
        return resolve_type_from_ty_node(&ty_node, &mut type_ctx);
    }

    // No type found - return error
    Ty::error(Span::from(0..0))
}
