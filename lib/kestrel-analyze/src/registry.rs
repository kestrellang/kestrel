//! Analyzer registry — collects all registered analyzers by granularity.
//!
//! Built once at compiler startup, stored as an Arc component on the
//! root entity for access from queries.

use std::sync::Arc;

use crate::traits::{AnalyzerId, BodyCheck, CompilationCheck, DeclCheck};

/// Registry of all analyzers, organized by granularity.
pub struct AnalyzerRegistry {
    pub(crate) body_checks: Vec<Arc<dyn BodyCheck>>,
    pub(crate) decl_checks: Vec<Arc<dyn DeclCheck>>,
    pub(crate) compilation_checks: Vec<Arc<dyn CompilationCheck>>,
}

impl Default for AnalyzerRegistry {
    fn default() -> Self {
        Self::new()
    }
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
    pub fn find_body_check(&self, id: AnalyzerId) -> Option<&Arc<dyn BodyCheck>> {
        self.body_checks.iter().find(|a| a.id() == id)
    }

    /// Look up a decl check by analyzer ID.
    pub fn find_decl_check(&self, id: AnalyzerId) -> Option<&Arc<dyn DeclCheck>> {
        self.decl_checks.iter().find(|a| a.id() == id)
    }

    /// Look up a compilation check by analyzer ID.
    pub fn find_compilation_check(&self, id: AnalyzerId) -> Option<&Arc<dyn CompilationCheck>> {
        self.compilation_checks.iter().find(|a| a.id() == id)
    }
}

/// ECS component wrapper for the registry. Stored on the root entity.
#[derive(Clone)]
pub struct AnalyzerRegistryRef(pub Arc<AnalyzerRegistry>);

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    /// `DiagnosticDescriptor::id` is documented as unique, but nothing
    /// enforced it — eight E-codes ended up claimed by two unrelated
    /// analyzers. Walk every registered analyzer's descriptors and reject
    /// any id declared twice, so a fresh allocation can't silently collide.
    /// Two analyzers may share ONE descriptor object (e.g. GenericsAnalyzer
    /// and TypeArgArityAnalyzer both list E438 from the same static array);
    /// only distinct descriptors with the same id are collisions.
    #[test]
    fn descriptor_ids_are_unique_across_all_analyzers() {
        let registry = crate::default_analyzers();
        let all = registry
            .body_checks
            .iter()
            .map(|a| a.descriptors())
            .chain(registry.decl_checks.iter().map(|a| a.descriptors()))
            .chain(registry.compilation_checks.iter().map(|a| a.descriptors()))
            .flatten();
        let mut owner: HashMap<&str, &crate::diagnostic::DiagnosticDescriptor> = HashMap::new();
        for d in all {
            if let Some(prev) = owner.insert(d.id, d) {
                assert!(
                    std::ptr::eq(prev, d),
                    "diagnostic id {} is declared by both `{}` and `{}` — \
                     allocate a fresh code (see kestrel-analyze/AGENTS.md)",
                    d.id,
                    prev.name,
                    d.name
                );
            }
        }
    }
}
