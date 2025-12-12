pub mod analyzer;
pub mod context;
mod runner;
mod walker;

pub mod analyzers {
    // Placeholder module for concrete analyzers moved from builder
    pub mod r#mod {}
}

pub mod diagnostics {
    // Placeholder module for diagnostics moved from builder
    pub mod r#mod {}
}

pub use analyzer::Analyzer;
pub use context::AnalysisContext;
pub use runner::{run, run_all};

