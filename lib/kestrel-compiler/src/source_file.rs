use kestrel_syntax_tree::SyntaxNode;

/// Represents a compiled source file.
///
/// Contains the original source code and the parsed syntax tree.
/// The semantic tree is stored at the compilation level for cross-file analysis.
pub struct SourceFile {
    name: String,
    source: String,
    syntax_tree: SyntaxNode,
}

impl SourceFile {
    /// Create a new source file.
    pub(crate) fn new(name: String, source: String, syntax_tree: SyntaxNode) -> Self {
        Self {
            name,
            source,
            syntax_tree,
        }
    }

    /// Get the file name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the source code.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Get the syntax tree.
    pub fn syntax_tree(&self) -> &SyntaxNode {
        &self.syntax_tree
    }
}
