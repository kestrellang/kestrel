use std::sync::Arc;

use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::field::FieldSymbol;
use kestrel_semantic_tree::symbol::{GetterSymbol, SetterSymbol};
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Spanned;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::{Symbol, SymbolId};

use kestrel_syntax_tree::utils::{
    extract_name, extract_visibility, find_child, get_node_span, get_visibility_span,
};

use crate::builder::Builder;
use kestrel_semantic_tree::behavior::visibility::{Visibility, find_visibility_scope};

/// Builder for field declarations.
pub struct FieldBuilder;

impl Builder for FieldBuilder {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        file_id: usize,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        let name_str = extract_name(syntax)?;
        let name_node = find_child(syntax, SyntaxKind::Name)?;
        let name_span = get_node_span(&name_node, file_id);

        let full_span = get_node_span(syntax, file_id);

        let visibility_str = extract_visibility(syntax);
        let visibility_enum = visibility_str.as_deref().and_then(Visibility::from_keyword);

        let visibility_span = get_visibility_span(syntax, file_id).unwrap_or(name_span.clone());
        let visibility_scope = find_visibility_scope(visibility_enum.as_ref(), parent, root);
        let visibility_behavior =
            VisibilityBehavior::new(visibility_enum, visibility_span, visibility_scope);

        let is_static = syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::StaticModifier);

        let is_mutable = syntax
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .any(|tok| tok.kind() == SyntaxKind::Var);

        // Check if this is a computed property (has PropertyAccessors)
        let property_accessors = syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::PropertyAccessors);
        let is_computed = property_accessors.is_some();

        let field_type = Ty::error(full_span.clone());
        let name = Spanned::new(name_str.clone(), name_span.clone());

        let field_symbol = FieldSymbol::new(
            name,
            full_span.clone(),
            visibility_behavior,
            is_static,
            is_mutable,
            is_computed,
            field_type,
            parent.cloned(),
        );
        let field_arc = Arc::new(field_symbol);
        let field_arc_dyn = field_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

        // If computed, create child getter/setter symbols
        if let Some(accessors) = property_accessors {
            // Find getter clause (explicit get { } syntax)
            let getter_clause = accessors
                .children()
                .find(|child| child.kind() == SyntaxKind::GetterClause);

            // Check for shorthand syntax (just a CodeBlock directly in PropertyAccessors)
            let shorthand_body = accessors
                .children()
                .find(|child| child.kind() == SyntaxKind::CodeBlock);

            // Check for protocol requirement syntax (just Get token without body)
            let has_get_token = accessors
                .children_with_tokens()
                .filter_map(|elem| elem.into_token())
                .any(|tok| tok.kind() == SyntaxKind::Get);

            // Create getter symbol for explicit, shorthand, or protocol requirement form
            if let Some(getter_node) = getter_clause {
                // Explicit: var x: Int { get { ... } }
                let getter_span = get_node_span(&getter_node, file_id);
                let getter_symbol = GetterSymbol::new(
                    SymbolId::new(),
                    &field_arc_dyn,
                    &name_str,
                    name_span.clone(),
                    getter_span,
                );
                let getter_arc = Arc::new(getter_symbol) as Arc<dyn Symbol<KestrelLanguage>>;
                field_arc.metadata().add_child(&getter_arc);
            } else if let Some(body_node) = shorthand_body {
                // Shorthand: var x: Int { expr }
                let getter_span = get_node_span(&body_node, file_id);
                let getter_symbol = GetterSymbol::new(
                    SymbolId::new(),
                    &field_arc_dyn,
                    &name_str,
                    name_span.clone(),
                    getter_span,
                );
                let getter_arc = Arc::new(getter_symbol) as Arc<dyn Symbol<KestrelLanguage>>;
                field_arc.metadata().add_child(&getter_arc);
            } else if has_get_token {
                // Protocol requirement: var x: Int { get }
                // Use the field's name span for the getter since there's no body
                let getter_symbol = GetterSymbol::new(
                    SymbolId::new(),
                    &field_arc_dyn,
                    &name_str,
                    name_span.clone(),
                    name_span.clone(),
                );
                let getter_arc = Arc::new(getter_symbol) as Arc<dyn Symbol<KestrelLanguage>>;
                field_arc.metadata().add_child(&getter_arc);
            }

            // Find setter clause (optional, only in explicit form)
            let setter_clause = accessors
                .children()
                .find(|child| child.kind() == SyntaxKind::SetterClause);

            // Check for protocol requirement setter (just Set token without body)
            let has_set_token = accessors
                .children_with_tokens()
                .filter_map(|elem| elem.into_token())
                .any(|tok| tok.kind() == SyntaxKind::Set);

            if let Some(setter_node) = setter_clause {
                let setter_span = get_node_span(&setter_node, file_id);
                let setter_symbol = SetterSymbol::new(
                    SymbolId::new(),
                    &field_arc_dyn,
                    &name_str,
                    name_span.clone(),
                    setter_span,
                );
                let setter_arc = Arc::new(setter_symbol) as Arc<dyn Symbol<KestrelLanguage>>;
                field_arc.metadata().add_child(&setter_arc);
            } else if has_set_token && setter_clause.is_none() {
                // Protocol requirement: var x: Int { get set }
                // Use the field's name span for the setter since there's no body
                let setter_symbol = SetterSymbol::new(
                    SymbolId::new(),
                    &field_arc_dyn,
                    &name_str,
                    name_span.clone(),
                    name_span.clone(),
                );
                let setter_arc = Arc::new(setter_symbol) as Arc<dyn Symbol<KestrelLanguage>>;
                field_arc.metadata().add_child(&setter_arc);
            }
        }

        if let Some(parent) = parent {
            parent.metadata().add_child(&field_arc_dyn);
        }

        Some(field_arc)
    }
}
