//! # Module Declaration Analyzer
//!
//! Every file that declares something needs a `module` line. A file without
//! one has its declarations built straight under the root, where every
//! module's name lookups end up, so its names leak into other modules: a
//! user `struct A` shadowed the `A` in std's `extend (A, B): Equatable`.
//!
//! ## Diagnostics
//!
//! ### E414 -- `missing_module_declaration` (Error, Correctness)
//! **Message:** "this file has no `module` declaration"

use std::collections::HashMap;

use crate::context::CompilationContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, CompilationCheck, Describe};
use crate::util;
use kestrel_ast_builder::{FileId, NodeKind};
use kestrel_hecs::Entity;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E414",
    name: "missing_module_declaration",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ModuleDeclarationAnalyzer;

impl Describe for ModuleDeclarationAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::ModuleDeclaration
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl CompilationCheck for ModuleDeclarationAnalyzer {
    fn check(&self, cx: &CompilationContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // A declaration sits directly under the root only when its file has
        // no `module` line. Report each such file once, at its first one.
        let mut first_by_file: HashMap<Entity, Entity> = HashMap::new();
        for &child in cx.query.children_of(cx.root) {
            if cx.query.get::<NodeKind>(child) == Some(&NodeKind::Module) {
                continue;
            }
            let Some(FileId(file)) = cx.query.get::<FileId>(child) else {
                continue;
            };
            first_by_file
                .entry(*file)
                .and_modify(|first| {
                    if span_start(cx, child) < span_start(cx, *first) {
                        *first = child;
                    }
                })
                .or_insert(child);
        }
        let mut firsts: Vec<Entity> = first_by_file.into_values().collect();
        firsts.sort_by_key(|&e| (util::entity_span(cx.query, e).file_id, span_start(cx, e)));
        firsts
            .into_iter()
            .map(|e| missing_module_diag(cx, e))
            .collect()
    }
}

fn span_start(cx: &CompilationContext<'_>, entity: Entity) -> usize {
    util::entity_span(cx.query, entity).start
}

fn missing_module_diag(cx: &CompilationContext<'_>, entity: Entity) -> AnalyzeDiagnostic {
    AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[0].id,
        severity: DESCRIPTORS[0].default_severity,
        message: "this file has no `module` declaration".into(),
        labels: vec![DiagLabel {
            span: util::entity_span(cx.query, entity),
            message: "declared outside any module".into(),
            is_primary: true,
        }],
        notes: vec!["start the file with a `module` line, such as `module Main`".into()],
    }
}
