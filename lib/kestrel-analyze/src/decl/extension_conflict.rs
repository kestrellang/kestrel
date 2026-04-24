//! Extension conflict detection moved to compilation/extension_conflict.rs
//! (CompilationCheck — needs cross-extension aggregation).
//!
//! This module is kept as a no-op DeclCheck for backward compatibility
//! with the registration in lib.rs. Can be removed once the old registration
//! is updated.

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use kestrel_ast_builder::NodeKind;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[];

pub struct ExtensionConflictAnalyzer;

impl Describe for ExtensionConflictAnalyzer {
    fn id(&self) -> &'static str {
        "extension_conflict_decl_stub"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for ExtensionConflictAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[]
    }

    fn check(&self, _cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        vec![]
    }
}
