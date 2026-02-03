use kestrel_syntax_tree::SyntaxNode;
use std::path::PathBuf;

/// Represents a compiled source file.
///
/// Contains the original source code and the parsed syntax tree.
/// The semantic tree is stored at the compilation level for cross-file analysis.
pub struct SourceFile {
    name: String,
    source: String,
    syntax_tree: SyntaxNode,
    /// The full path to the source file, if it was loaded from disk.
    path: Option<PathBuf>,
}

impl SourceFile {
    /// Create a new source file.
    pub(crate) fn new(name: String, source: String, syntax_tree: SyntaxNode) -> Self {
        Self {
            name,
            source,
            syntax_tree,
            path: None,
        }
    }

    /// Create a new source file with a path.
    pub(crate) fn with_path(
        name: String,
        source: String,
        syntax_tree: SyntaxNode,
        path: PathBuf,
    ) -> Self {
        Self {
            name,
            source,
            syntax_tree,
            path: Some(path),
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

    /// Get the full path to the source file, if available.
    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    /// Get the directory containing this source file, if path is available.
    pub fn directory(&self) -> Option<PathBuf> {
        self.path.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf()))
    }
}
