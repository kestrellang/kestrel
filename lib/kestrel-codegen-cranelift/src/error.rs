use std::fmt;

#[derive(Debug)]
pub enum CodegenError {
    ModuleCreation(String),
    FunctionCompilation {
        name: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    FunctionDefinition {
        name: String,
        source: cranelift_module::ModuleError,
    },
    ModuleFinish(String),
    LinkerError(String),
    IoError(std::io::Error),
    Unsupported(String),
    DataSection(String),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModuleCreation(msg) => write!(f, "module creation failed: {msg}"),
            Self::FunctionCompilation { name, source } => {
                write!(f, "failed to compile function '{name}': {source}")
            }
            Self::FunctionDefinition { name, source } => {
                write!(f, "failed to define function '{name}': {source}")
            }
            Self::ModuleFinish(msg) => write!(f, "module finish failed: {msg}"),
            Self::LinkerError(msg) => write!(f, "linker error: {msg}"),
            Self::IoError(e) => write!(f, "I/O error: {e}"),
            Self::Unsupported(msg) => write!(f, "unsupported: {msg}"),
            Self::DataSection(msg) => write!(f, "data section error: {msg}"),
        }
    }
}

impl std::error::Error for CodegenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::FunctionCompilation { source, .. } => Some(source.as_ref()),
            Self::FunctionDefinition { source, .. } => Some(source),
            Self::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CodegenError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}
