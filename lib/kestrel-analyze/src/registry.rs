//! Analyzer registry — collects all registered analyzers by granularity.
//!
//! Built once at compiler startup, stored as an Arc component on the
//! root entity for access from queries.

use std::sync::Arc;

use crate::traits::{BodyCheck, CompilationCheck, DeclCheck};

/// Registry of all analyzers, organized by granularity.
pub struct AnalyzerRegistry {
    pub(crate) body_checks: Vec<Arc<dyn BodyCheck>>,
    pub(crate) decl_checks: Vec<Arc<dyn DeclCheck>>,
    pub(crate) compilation_checks: Vec<Arc<dyn CompilationCheck>>,
}

impl AnalyzerRegistry {
    pub fn new() -> Self {
        Self {
            body_checks: Vec::new(),
            decl_checks: Vec::new(),
            compilation_checks: Vec::new(),
        }
    }

    pub fn add_body_check(&mut self, analyzer: impl BodyCheck) {
        self.body_checks.push(Arc::new(analyzer));
    }

    pub fn add_decl_check(&mut self, analyzer: impl DeclCheck) {
        self.decl_checks.push(Arc::new(analyzer));
    }

    pub fn add_compilation_check(&mut self, analyzer: impl CompilationCheck) {
        self.compilation_checks.push(Arc::new(analyzer));
    }

    /// Look up a body check by analyzer ID.
    pub fn find_body_check(&self, id: &str) -> Option<&Arc<dyn BodyCheck>> {
        self.body_checks.iter().find(|a| a.id() == id)
    }

    /// Look up a decl check by analyzer ID.
    pub fn find_decl_check(&self, id: &str) -> Option<&Arc<dyn DeclCheck>> {
        self.decl_checks.iter().find(|a| a.id() == id)
    }

    /// Look up a compilation check by analyzer ID.
    pub fn find_compilation_check(&self, id: &str) -> Option<&Arc<dyn CompilationCheck>> {
        self.compilation_checks.iter().find(|a| a.id() == id)
    }
}

/// ECS component wrapper for the registry. Stored on the root entity.
#[derive(Clone)]
pub struct AnalyzerRegistryRef(pub Arc<AnalyzerRegistry>);
