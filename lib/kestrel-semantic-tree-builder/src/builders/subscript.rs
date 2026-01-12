//! Builder for subscript declarations.
//!
//! This module builds `SubscriptSymbol` instances from syntax trees.
//! Subscripts are like computed properties but accept parameters, enabling
//! indexed or keyed access patterns like `array(0)` or `dictionary("key")`.

use std::sync::Arc;

use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::subscript::SubscriptSymbol;
use kestrel_semantic_tree::symbol::{GetterSymbol, SetterSymbol};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::{Symbol, SymbolId};

use kestrel_syntax_tree::utils::{extract_visibility, find_child, get_node_span, get_visibility_span};

use crate::builder::Builder;
use crate::builders::type_parameter::{add_type_params_as_children, extract_type_parameters};
use kestrel_semantic_tree::behavior::visibility::{Visibility, find_visibility_scope};
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;

/// Builder for subscript declarations.
///
/// Creates `SubscriptSymbol` with child getter/setter symbols from syntax.
/// Subscripts can only appear in struct, enum, protocol, or extension declarations.
pub struct SubscriptBuilder;

impl Builder for SubscriptBuilder {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        file_id: usize,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        let parent = parent?;

        // Subscripts are only valid in struct, enum, protocol, extension
        let parent_kind = parent.metadata().kind();
        if !matches!(
            parent_kind,
            KestrelSymbolKind::Struct
                | KestrelSymbolKind::Protocol
                | KestrelSymbolKind::Enum
                | KestrelSymbolKind::Extension
        ) {
            return None;
        }

        // Extract visibility
        let visibility_str = extract_visibility(syntax);
        let visibility_enum = visibility_str.as_deref().and_then(Visibility::from_keyword);

        // Get spans
        let full_span = get_node_span(syntax, file_id);
        let declaration_span = find_subscript_keyword_span(syntax, file_id)
            .unwrap_or_else(|| full_span.clone());
        let visibility_span = get_visibility_span(syntax, file_id)
            .unwrap_or_else(|| declaration_span.clone());

        let visibility_scope = find_visibility_scope(visibility_enum.as_ref(), Some(parent), root);
        let visibility_behavior =
            VisibilityBehavior::new(visibility_enum, visibility_span, visibility_scope);

        // Check for static modifier
        let is_static = syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::StaticModifier);

        // Create subscript symbol
        let subscript_symbol = SubscriptSymbol::new(
            SymbolId::new(),
            full_span.clone(),
            declaration_span.clone(),
            visibility_behavior,
            is_static,
            Some(parent.clone()),
        );
        let subscript_arc = Arc::new(subscript_symbol);
        let subscript_dyn = subscript_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

        // Build type parameters as children (if generic)
        let type_parameters =
            extract_type_parameters(syntax, source, file_id, Some(subscript_dyn.clone()));
        add_type_params_as_children(&type_parameters, &subscript_dyn);

        // Build getter/setter children based on body
        if let Some(body) = find_child(syntax, SyntaxKind::SubscriptBody) {
            build_subscript_accessors(&body, file_id, &subscript_dyn, &declaration_span);
        }

        // Register with parent
        parent.metadata().add_child(&subscript_dyn);

        Some(subscript_arc)
    }

    /// Subscripts are terminal - we handle building children ourselves
    fn is_terminal(&self) -> bool {
        true
    }
}

/// Find the span of the `subscript` keyword in the syntax
fn find_subscript_keyword_span(syntax: &SyntaxNode, file_id: usize) -> Option<Span> {
    for child in syntax.children_with_tokens() {
        if let Some(token) = child.into_token() {
            if token.kind() == SyntaxKind::Subscript {
                let text_range = token.text_range();
                return Some(Span::new(
                    file_id,
                    (text_range.start().into())..(text_range.end().into()),
                ));
            }
        }
    }
    None
}

/// Build getter and setter child symbols for a subscript
fn build_subscript_accessors(
    body: &SyntaxNode,
    file_id: usize,
    parent: &Arc<dyn Symbol<KestrelLanguage>>,
    declaration_span: &Span,
) {
    // Check for PropertyAccessors (explicit get/set or protocol requirements)
    if let Some(accessors) = find_child(body, SyntaxKind::PropertyAccessors) {
        // Check for explicit GetterClause
        let getter_clause = accessors
            .children()
            .find(|child| child.kind() == SyntaxKind::GetterClause);

        // Check for protocol requirement syntax (just Get token without body)
        let has_get_token = accessors
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .any(|tok| tok.kind() == SyntaxKind::Get);

        // Create getter symbol
        if let Some(getter_node) = getter_clause {
            // Explicit: subscript(...) -> T { get { ... } }
            let getter_span = get_node_span(&getter_node, file_id);
            let getter_symbol = GetterSymbol::new(
                SymbolId::new(),
                parent,
                "subscript",
                declaration_span.clone(),
                getter_span,
            );
            let getter_arc = Arc::new(getter_symbol) as Arc<dyn Symbol<KestrelLanguage>>;
            parent.metadata().add_child(&getter_arc);
        } else if has_get_token {
            // Protocol requirement: subscript(...) -> T { get } or { get set }
            let getter_symbol = GetterSymbol::new(
                SymbolId::new(),
                parent,
                "subscript",
                declaration_span.clone(),
                declaration_span.clone(),
            );
            let getter_arc = Arc::new(getter_symbol) as Arc<dyn Symbol<KestrelLanguage>>;
            parent.metadata().add_child(&getter_arc);
        }

        // Check for explicit SetterClause
        let setter_clause = accessors
            .children()
            .find(|child| child.kind() == SyntaxKind::SetterClause);

        // Check for protocol requirement setter (just Set token without body)
        let has_set_token = accessors
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .any(|tok| tok.kind() == SyntaxKind::Set);

        // Create setter symbol if present
        if let Some(setter_node) = setter_clause {
            // Explicit: subscript(...) -> T { get { } set { ... } }
            let setter_span = get_node_span(&setter_node, file_id);
            let setter_symbol = SetterSymbol::new(
                SymbolId::new(),
                parent,
                "subscript",
                declaration_span.clone(),
                setter_span,
            );
            let setter_arc = Arc::new(setter_symbol) as Arc<dyn Symbol<KestrelLanguage>>;
            parent.metadata().add_child(&setter_arc);
        } else if has_set_token {
            // Protocol requirement: subscript(...) -> T { get set }
            let setter_symbol = SetterSymbol::new(
                SymbolId::new(),
                parent,
                "subscript",
                declaration_span.clone(),
                declaration_span.clone(),
            );
            let setter_arc = Arc::new(setter_symbol) as Arc<dyn Symbol<KestrelLanguage>>;
            parent.metadata().add_child(&setter_arc);
        }
    } else {
        // Shorthand body - getter only: subscript(...) -> T { expr }
        // The body directly contains a CodeBlock
        let shorthand_body = find_child(body, SyntaxKind::CodeBlock);

        if let Some(body_node) = shorthand_body {
            let getter_span = get_node_span(&body_node, file_id);
            let getter_symbol = GetterSymbol::new(
                SymbolId::new(),
                parent,
                "subscript",
                declaration_span.clone(),
                getter_span,
            );
            let getter_arc = Arc::new(getter_symbol) as Arc<dyn Symbol<KestrelLanguage>>;
            parent.metadata().add_child(&getter_arc);
        } else {
            // Fallback: create getter with declaration span if no body found
            // This handles edge cases in the syntax tree
            let getter_symbol = GetterSymbol::new(
                SymbolId::new(),
                parent,
                "subscript",
                declaration_span.clone(),
                declaration_span.clone(),
            );
            let getter_arc = Arc::new(getter_symbol) as Arc<dyn Symbol<KestrelLanguage>>;
            parent.metadata().add_child(&getter_arc);
        }
    }
}
