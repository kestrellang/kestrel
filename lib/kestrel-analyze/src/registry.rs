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

/// Codes that are registered but deliberately not emitted yet.
///
/// A descriptor with no emit site is invisible drift — it is documented as a
/// live diagnostic, with worked examples, while the compiler cannot produce it
/// (E600 moved into the solver, E602 is a `// TODO`). Reserving is fine;
/// reserving *silently* is not. Anything here must say why, and anything not
/// here must be reachable.
pub(crate) const RESERVED_UNEMITTED: &[(&str, &str)] = &[
    (
        "E206",
        "let_to_consuming — subsumed by the move checker's E50x family",
    ),
    (
        "E303",
        "irrefutable_pattern_makes_arms_unreachable — E306 labels the dead \
         code and is the one emitted; E303 is held for the degenerate case \
         where no E306 exists (see exhaustiveness.rs)",
    ),
    (
        "E446",
        "associated_type_constraint_not_satisfied — check not written yet",
    ),
    (
        "E448",
        "type_alias_contains_infer — reserved with the type-alias plan",
    ),
    (
        "E502",
        "cloneable_field_requires_conformance — retired when clone shims \
         became automatic; the analyzer returns no diagnostics",
    ),
    (
        "E600",
        "closure capture-kind mismatch — the check moved into the solver, \
         where it renders as an InferError; the descriptor is kept so the \
         code is not reallocated under a different meaning",
    ),
    (
        "E602",
        "closure escape analysis — not implemented (closure.rs TODO)",
    ),
];

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

    /// Descriptor `name`s are the stable, human-readable handle for a code and
    /// appear in `docs/error-codes.md`. Two descriptors sharing a name means
    /// two codes claim the same fact — the shape "one analyzer per fact"
    /// (AGENTS.md) exists to prevent.
    #[test]
    fn descriptor_names_are_unique_across_all_analyzers() {
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
            if let Some(prev) = owner.insert(d.name, d) {
                assert!(
                    std::ptr::eq(prev, d),
                    "descriptor name `{}` is claimed by both {} and {} — \
                     two codes for one fact",
                    d.name,
                    prev.id,
                    d.id
                );
            }
        }
    }

    /// `find_body_check` / `find_decl_check` / `find_compilation_check` are
    /// linear first-match by `AnalyzerId`. A copy-pasted analyzer that forgets
    /// to add a new `AnalyzerId` variant is therefore **never run** while its
    /// twin runs twice — silently, and with no diagnostic of its own. That is
    /// the same class of bug the descriptor-uniqueness test above catches one
    /// level up, so it gets the same treatment.
    #[test]
    fn analyzer_ids_are_unique_within_each_list() {
        let registry = crate::default_analyzers();
        let lists: [(&str, Vec<crate::traits::AnalyzerId>); 3] = [
            (
                "body_checks",
                registry.body_checks.iter().map(|a| a.id()).collect(),
            ),
            (
                "decl_checks",
                registry.decl_checks.iter().map(|a| a.id()).collect(),
            ),
            (
                "compilation_checks",
                registry.compilation_checks.iter().map(|a| a.id()).collect(),
            ),
        ];
        for (list, ids) in lists {
            let mut seen = std::collections::HashSet::new();
            for id in ids {
                assert!(
                    seen.insert(id),
                    "{list} registers {id:?} twice — the second registration is \
                     unreachable through find_*, so that analyzer never runs"
                );
            }
        }
    }

    /// Every registered code is either reachable or an acknowledged
    /// reservation, and every reservation names a code that still exists.
    ///
    /// Reachability itself is enforced at emit time by `assert_owned` in
    /// `lib.rs` (an analyzer may only emit codes it declares); this test closes
    /// the other direction — a *declared* code that nothing emits, which is how
    /// E600 and E602 came to be documented as live diagnostics the compiler
    /// cannot produce.
    #[test]
    fn reservations_name_real_descriptors() {
        let registry = crate::default_analyzers();
        let all: std::collections::HashSet<&str> = registry
            .body_checks
            .iter()
            .map(|a| a.descriptors())
            .chain(registry.decl_checks.iter().map(|a| a.descriptors()))
            .chain(registry.compilation_checks.iter().map(|a| a.descriptors()))
            .flatten()
            .map(|d| d.id)
            .collect();
        for (id, why) in super::RESERVED_UNEMITTED {
            assert!(
                all.contains(id),
                "{id} is listed as reserved ({why}) but no analyzer declares it — \
                 delete the reservation or restore the descriptor"
            );
        }
    }

    /// Every registered code appears in `docs/error-codes.md`.
    ///
    /// The doc opens by claiming "every code below corresponds to a descriptor
    /// or emit site". Nothing checked it in either direction, so nine emitted
    /// codes went undocumented while ~37 fabricated `E05xx`/`E06xx` codes were
    /// documented with worked transcripts (F16). This pins the registry half;
    /// the codespan half (E100, E48x, E49x, E5xx) is emitted outside this crate
    /// and is not visible from here.
    #[test]
    fn every_registered_code_is_documented() {
        let doc_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/error-codes.md");
        let Ok(doc) = std::fs::read_to_string(doc_path) else {
            // Docs aren't shipped with the crate; skip rather than fail a
            // packaged build.
            return;
        };
        let registry = crate::default_analyzers();
        let mut missing: Vec<&str> = registry
            .body_checks
            .iter()
            .map(|a| a.descriptors())
            .chain(registry.decl_checks.iter().map(|a| a.descriptors()))
            .chain(registry.compilation_checks.iter().map(|a| a.descriptors()))
            .flatten()
            .map(|d| d.id)
            .filter(|id| !doc.contains(*id))
            .collect();
        missing.sort_unstable();
        missing.dedup();
        assert!(
            missing.is_empty(),
            "these registered codes are missing from docs/error-codes.md: {missing:?}"
        );
    }
}
