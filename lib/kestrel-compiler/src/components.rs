/// Raw source content of a file entity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SourceText(pub String);

pub use kestrel_ast_builder::FilePath;
