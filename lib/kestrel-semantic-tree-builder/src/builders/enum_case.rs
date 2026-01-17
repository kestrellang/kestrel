use std::sync::Arc;

use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::enum_case::EnumCaseSymbol;
use kestrel_span::Spanned;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use kestrel_syntax_tree::utils::{extract_name, find_child, get_node_span};

use crate::builder::Builder;
use kestrel_semantic_tree::behavior::visibility::{Visibility, find_visibility_scope};

/// Builder for enum case declarations.
///
/// Enum cases inherit visibility from their parent enum.
/// CallableBehavior (for cases with associated values) is added during the bind phase,
/// not here in the build phase.
pub struct EnumCaseBuilder;

impl Builder for EnumCaseBuilder {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        _source: &str,
        file_id: usize,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        let name_str = extract_name(syntax)?;
        let name_node = find_child(syntax, SyntaxKind::Name)?;
        let name_span = get_node_span(&name_node, file_id);

        let full_span = get_node_span(syntax, file_id);

        // Enum cases inherit visibility from the parent enum
        let parent_visibility = parent
            .and_then(|p| p.metadata().get_behavior::<VisibilityBehavior>())
            .and_then(|v| v.visibility().copied());

        let visibility_scope = find_visibility_scope(parent_visibility.as_ref(), parent, root);
        let visibility_behavior = VisibilityBehavior::new(
            parent_visibility,
            name_span.clone(),
            visibility_scope,
        );

        let name = Spanned::new(name_str, name_span);

        let case_symbol =
            EnumCaseSymbol::new(name, full_span, visibility_behavior, parent.cloned());
        let case_arc = Arc::new(case_symbol);
        let case_arc_dyn = case_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

        // Add case to parent's children (should be an EnumSymbol)
        if let Some(parent) = parent {
            parent.metadata().add_child(&case_arc_dyn);
        }

        Some(case_arc)
    }

    /// Enum cases are terminal - they don't have children that need walking.
    /// The parameters (associated values) are processed during the bind phase.
    fn is_terminal(&self) -> bool {
        true
    }
}
