use std::sync::Arc;

use kestrel_semantic_tree::behavior::ComputedMemberAccessBehavior;
use kestrel_semantic_tree::behavior::member_access::MemberAccessBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::field::FieldSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::resolution::type_resolver::{TypeSyntaxContext, resolve_type_from_ty_node};

/// Binder for field declarations
pub struct FieldBinder;

impl DeclarationBinder for FieldBinder {
    fn bind_signature(
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
        let file_id = context.file_id_for_symbol(symbol);

        // Resolve attributes
        let attributes_behavior = crate::binders::utils::attributes::resolve_attributes(
            syntax,
            &source,
            file_id,
            context.diagnostics,
        );
        symbol.metadata().add_behavior(attributes_behavior);

        // Resolve the type directly from syntax
        let resolved_type =
            resolve_field_type_from_syntax(syntax, &source, file_id, symbol_id, context);

        // Add a TypedBehavior with the resolved type
        let typed_behavior = TypedBehavior::new(resolved_type.clone(), span);
        symbol.metadata().add_behavior(typed_behavior);

        // Add appropriate member access behavior so this field can be accessed via dot notation
        let field_name = symbol.metadata().name().value.clone();

        // Get field properties from the FieldSymbol
        if let Some(field_symbol) = symbol.as_ref().downcast_ref::<FieldSymbol>() {
            if field_symbol.is_computed() {
                // Computed property: use ComputedMemberAccessBehavior
                let getter_id = field_symbol
                    .getter()
                    .expect("computed property must have a getter");
                let setter_id = field_symbol.setter();
                let computed_behavior = ComputedMemberAccessBehavior::new(
                    field_name,
                    resolved_type,
                    getter_id,
                    setter_id,
                );
                symbol.metadata().add_behavior(computed_behavior);
            } else {
                // Stored field: use MemberAccessBehavior
                let is_mutable = field_symbol.is_mutable();
                let member_access_behavior =
                    MemberAccessBehavior::new(field_name, resolved_type, is_mutable);
                symbol.metadata().add_behavior(member_access_behavior);
            }
        } else {
            // Fallback: treat as immutable stored field
            let member_access_behavior =
                MemberAccessBehavior::new(field_name, resolved_type, false);
            symbol.metadata().add_behavior(member_access_behavior);
        }
    }
}

/// Resolve the field type from a FieldDeclaration syntax node
/// This extracts the type from syntax and immediately resolves it
fn resolve_field_type_from_syntax(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
) -> Ty {
    // Find the Ty node and resolve using shared utility
    if let Some(ty_node) = syntax
        .children()
        .find(|child| child.kind() == SyntaxKind::Ty)
    {
        let mut type_ctx =
            TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
        return resolve_type_from_ty_node(&ty_node, &mut type_ctx);
    }

    // No type found - return error
    Ty::error(Span::new(file_id, 0..0))
}
