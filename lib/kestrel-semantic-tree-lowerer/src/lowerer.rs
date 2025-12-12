use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_model::SemanticModel;
use kestrel_syntax_tree::SyntaxNode;

/// A single input file for semantic lowering.
pub struct BuildFile<'a> {
    pub file_name: &'a str,
    pub syntax: &'a SyntaxNode,
    pub source: &'a str,
    pub file_id: usize,
}

/// Lower one or more syntax trees into a `SemanticModel`.
pub fn build<'a, I>(files: I, diagnostics: &mut DiagnosticContext) -> SemanticModel
where
    I: IntoIterator<Item = BuildFile<'a>>,
{
    let mut builder = kestrel_semantic_tree_builder::SemanticTreeBuilder::new();

    for file in files {
        builder.add_file(
            file.file_name,
            file.syntax,
            file.source,
            diagnostics,
            file.file_id,
        );
    }

    let tree = builder.build();
    let (root, syntax_map, sources) = tree.into_parts();
    SemanticModel::new(root, syntax_map, sources)
}

