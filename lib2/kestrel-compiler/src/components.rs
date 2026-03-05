/// Raw source content of a file entity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SourceText(pub String);

/// Display path for diagnostics rendering.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FilePath(pub String);
