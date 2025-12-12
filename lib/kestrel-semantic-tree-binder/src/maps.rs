use std::collections::HashMap;

use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::symbol::SymbolId;

/// Storage for source code by file name.
pub type SourceMap = HashMap<String, String>;

/// Storage for syntax nodes by symbol ID, allowing bind phase to access syntax.
pub type SyntaxMap = HashMap<SymbolId, SyntaxNode>;

