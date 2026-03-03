//! DocCommentBehavior for storing doc comments on symbols.

use semantic_tree::behavior::Behavior;

use crate::{behavior::KestrelBehaviorKind, language::KestrelLanguage};

/// Behavior that stores a doc comment on a symbol.
///
/// Doc comments are `///` line comments or `/** */` block comments
/// that appear immediately before a declaration. The text is stored
/// with markers stripped and lines joined.
#[derive(Debug, Clone)]
pub struct DocCommentBehavior {
    text: String,
}

impl Behavior<KestrelLanguage> for DocCommentBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::DocComment
    }
}

impl DocCommentBehavior {
    /// Create a new DocCommentBehavior with the given text.
    pub fn new(text: String) -> Self {
        Self { text }
    }

    /// Get the doc comment text.
    pub fn text(&self) -> &str {
        &self.text
    }
}
