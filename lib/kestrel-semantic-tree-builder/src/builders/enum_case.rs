use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::enum_case::EnumCaseSymbol;
use kestrel_span::Spanned;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use kestrel_syntax_tree::utils::{extract_name, find_child, get_node_span};

use crate::builder::Builder;

/// Builder for enum case declarations.
///
/// This builder creates the basic `EnumCaseSymbol` during the build phase.
/// If the case has associated values (parameters), `CallableBehavior` will be
/// added during the bind phase by the EnumCaseBinder.
pub struct EnumCaseBuilder;

impl Builder for EnumCaseBuilder {
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

        let name = Spanned::new(name_str, name_span);

        let case_symbol = EnumCaseSymbol::new(name, full_span, parent.cloned());
        let case_arc = Arc::new(case_symbol);
        let case_arc_dyn = case_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

        if let Some(parent) = parent {
            parent.metadata().add_child(&case_arc_dyn);
        }

        Some(case_arc)
    }
}
