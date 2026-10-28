//! # Import Target Analyzer
//!
//! An `import` must name a module that exists (or, as `import A.B.T`, a
//! member of one the importing file can see), and each item of a selective
//! import (`import A.(Foo)`) must name a member the importing file can see.
//! Unresolved imports used to be dropped silently, so a typo compiled as long
//! as nothing used the name.
//!
//! ## Diagnostics
//!
//! ### E629 -- `unresolved_import` (Error, Correctness)
//!
//! **Message:** "cannot find '{path}'" (neither a module nor a visible
//! member of one) or "module '{path}' has no
//! member '{name}'"
//!
//! **Labels:**
//! - Primary: the import declaration
//!   - Span source: the import's `DeclSpan`
//!   - Message: "not found"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use kestrel_ast_builder::{DeclSpan, ImportItems, ModulePath, NodeKind};
use kestrel_name_res::VisibleChildrenByName;
use kestrel_name_res::scope::{ImportTarget, resolve_import_path};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E629",
    name: "unresolved_import",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ImportTargetAnalyzer;

impl Describe for ImportTargetAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::ImportTarget
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for ImportTargetAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Import]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let (Some(path), Some(span)) = (
            cx.query.get::<ModulePath>(cx.entity),
            cx.query.get::<DeclSpan>(cx.entity),
        ) else {
            return vec![];
        };
        let dotted = path.0.join(".");
        // The same resolution `process_import` uses (single source of truth).
        let module = match resolve_import_path(cx.query, &path.0, cx.entity, cx.root) {
            ImportTarget::Module(module) => module,
            ImportTarget::Member { .. } => return vec![],
            ImportTarget::Unresolved => {
                return vec![error(span, format!("cannot find '{dotted}'"))];
            },
        };
        let Some(items) = cx.query.get::<ImportItems>(cx.entity) else {
            return vec![];
        };
        items
            .0
            .iter()
            .filter(|item| {
                cx.query
                    .query(VisibleChildrenByName {
                        parent: module,
                        name: item.name.clone(),
                        context: cx.entity,
                    })
                    .is_empty()
            })
            .map(|item| {
                error(
                    span,
                    format!("module '{dotted}' has no member '{}'", item.name),
                )
            })
            .collect()
    }
}

fn error(span: &DeclSpan, message: String) -> AnalyzeDiagnostic {
    AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[0].id,
        severity: DESCRIPTORS[0].default_severity,
        message,
        labels: vec![DiagLabel {
            span: span.0.clone(),
            message: "not found".into(),
            is_primary: true,
        }],
        notes: vec![],
    }
}
