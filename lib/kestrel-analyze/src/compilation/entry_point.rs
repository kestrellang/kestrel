//! # Entry Point Analyzer
//!
//! Validates the `@main` attribute that marks a program's entry point. Replaces
//! the old discover-by-name scheme (a free function literally called `main`):
//! the entry point is now whichever free function carries `@main`, regardless
//! of name. Runs as a `CompilationCheck` because the rules are whole-program
//! (how many `@main`s exist) and must see every entity.
//!
//! All four diagnostics only have an effect when a `@main` is actually written
//! (E615/E616/E617) or when an executable is being built (E618), so they never
//! fire on the large body of existing diagnostics tests / libraries that have no
//! `@main`.
//!
//! ## Diagnostics
//!
//! ### E615 — `main_not_free_function` (Error, Correctness)
//!
//! `@main` must sit on a free (module-level) function. Firing on a method,
//! static method, type, field, etc.
//!
//! ### E616 — `invalid_main_return_type` (Error, Correctness)
//!
//! A `@main` function must return `()` (Void) or an internal primitive integer
//! (`lang.i8`/`i16`/`i32`/`i64`). The stdlib struct integers (`Int64`, …),
//! floats, strings, tuples, etc. are rejected — the entry point speaks the raw
//! C-ABI boundary.
//!
//! ### E617 — `multiple_main` (Error, Correctness)
//!
//! More than one `@main` in the compilation. Labeled on each offender.
//!
//! ### E618 — `missing_main` (Error, Correctness)
//!
//! An executable build (`cx.is_executable`) with no `@main` at all. Gated on
//! build mode so non-executable compilations (libraries, `kestrel check`, the
//! LSP, diagnostics tests) never require an entry point. Anchored to the entry
//! module's declaration.

use crate::context::CompilationContext;
use crate::diagnostic::*;
use crate::traits::{CompilationCheck, Describe};
use crate::util;
use kestrel_ast::AstType;
use kestrel_ast_builder::{Attributes, DeclSpan, NodeKind, TypeAnnotation};
use kestrel_hecs::Entity;
use kestrel_name_res::{ResolveTypePath, TypeResolution};
use kestrel_span::Span;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E615",
        name: "main_not_free_function",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E616",
        name: "invalid_main_return_type",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E617",
        name: "multiple_main",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E618",
        name: "missing_main",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct EntryPointAnalyzer;

impl Describe for EntryPointAnalyzer {
    fn id(&self) -> &'static str {
        "entry_point"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl CompilationCheck for EntryPointAnalyzer {
    fn check(&self, cx: &CompilationContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut mains = Vec::new();
        collect_mains(cx, cx.root, &mut mains);

        // No `@main` anywhere: only an executable build needs one (E618).
        if mains.is_empty() {
            if cx.is_executable {
                return vec![missing_main_diag(cx)];
            }
            return vec![];
        }

        let mut diags = Vec::new();

        // More than one entry point (E617) — flag each.
        if mains.len() > 1 {
            for &entity in &mains {
                diags.push(multiple_main_diag(cx, entity));
            }
        }

        // Per-`@main` validity: free function (E615) + return type (E616).
        for &entity in &mains {
            if !is_free_function(cx, entity) {
                diags.push(not_free_function_diag(cx, entity));
                continue; // return-type check is meaningless on a non-function
            }
            if !main_return_type_ok(cx, entity) {
                diags.push(invalid_return_diag(cx, entity));
            }
        }

        diags
    }
}

// ===== Collection =====

/// Walk the entity tree collecting every entity that carries a `@main`
/// attribute (valid or not — validity is judged afterwards).
fn collect_mains(cx: &CompilationContext<'_>, entity: Entity, out: &mut Vec<Entity>) {
    if let Some(attrs) = cx.query.get::<Attributes>(entity)
        && attrs.0.iter().any(|a| a.name == "main")
    {
        out.push(entity);
    }
    for &child in cx.query.children_of(entity) {
        collect_mains(cx, child, out);
    }
}

// ===== Predicates =====

/// True iff `entity` is a free (module-level) function — the only valid `@main`
/// site. Methods, static methods, and `@main` on non-functions all fail this.
fn is_free_function(cx: &CompilationContext<'_>, entity: Entity) -> bool {
    if !matches!(cx.query.get::<NodeKind>(entity), Some(NodeKind::Function)) {
        return false;
    }
    // A free function's parent is a Module; a method's parent is the enclosing
    // Struct / Enum / Extension / Protocol.
    matches!(
        cx.query.parent_of(entity).and_then(|p| cx.query.get::<NodeKind>(p)),
        Some(NodeKind::Module)
    )
}

/// True iff the function's declared return type is acceptable for `@main`:
/// `()` (no annotation or `Unit`) or a `lang` primitive integer. An unresolved
/// named type is treated as acceptable here so the type-resolution error owns
/// the diagnostic instead of double-reporting.
fn main_return_type_ok(cx: &CompilationContext<'_>, entity: Entity) -> bool {
    let Some(TypeAnnotation(ty)) = cx.query.get::<TypeAnnotation>(entity) else {
        return true; // no `-> T` ⇒ Void
    };
    match ty {
        AstType::Unit(_) => true,
        AstType::Named { segments, .. } => {
            let context = cx.query.parent_of(entity).unwrap_or(cx.root);
            let segs: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
            match cx.query.query(ResolveTypePath {
                segments: segs,
                context,
                root: cx.root,
            }) {
                TypeResolution::Found(resolved) => is_lang_primitive_int(cx, resolved),
                _ => true, // unresolved — let the resolution error handle it
            }
        },
        _ => false, // tuple / function / optional / array / etc. — not a valid exit type
    }
}

/// True iff `resolved` is one of the `lang.iN` primitive integer entities
/// (`i8`/`i16`/`i32`/`i64`). These are seeded under the `lang` module and are
/// distinct from the stdlib `IntN` struct wrappers.
fn is_lang_primitive_int(cx: &CompilationContext<'_>, resolved: Entity) -> bool {
    for name in ["i8", "i16", "i32", "i64"] {
        if let TypeResolution::Found(e) = cx.query.query(ResolveTypePath {
            segments: vec!["lang".into(), name.into()],
            context: cx.root,
            root: cx.root,
        }) && e == resolved
        {
            return true;
        }
    }
    false
}

/// Span to anchor the whole-program "missing entry point" error on. Module
/// entities carry no `DeclSpan`, so we use the last declaration (in DFS order)
/// that has one: the user's program is built after stdlib, so the last
/// `DeclSpan` in the tree falls in the user's source rather than stdlib.
fn entry_module_span(cx: &CompilationContext<'_>) -> Span {
    let mut span: Option<Span> = None;
    last_decl_span(cx, cx.root, &mut span);
    span.unwrap_or_else(|| Span::synthetic(0))
}

fn last_decl_span(cx: &CompilationContext<'_>, entity: Entity, out: &mut Option<Span>) {
    if let Some(DeclSpan(s)) = cx.query.get::<DeclSpan>(entity) {
        *out = Some(s.clone());
    }
    for &child in cx.query.children_of(entity) {
        last_decl_span(cx, child, out);
    }
}

// ===== Diagnostic constructors =====

fn not_free_function_diag(cx: &CompilationContext<'_>, entity: Entity) -> AnalyzeDiagnostic {
    let name = util::entity_name(cx.query, entity);
    AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[0].id,
        severity: DESCRIPTORS[0].default_severity,
        message: format!("`@main` on '{name}' must be a free function"),
        labels: vec![DiagLabel {
            span: util::entity_span(cx.query, entity),
            message: "`@main` is only allowed on a free (module-level) function".into(),
            is_primary: true,
        }],
        notes: vec![],
    }
}

fn invalid_return_diag(cx: &CompilationContext<'_>, entity: Entity) -> AnalyzeDiagnostic {
    let name = util::entity_name(cx.query, entity);
    AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[1].id,
        severity: DESCRIPTORS[1].default_severity,
        message: format!("`@main` function '{name}' has an invalid return type"),
        labels: vec![DiagLabel {
            span: util::entity_span(cx.query, entity),
            message: "`@main` must return `()` or a primitive integer (lang.i8/i16/i32/i64)".into(),
            is_primary: true,
        }],
        notes: vec![
            "the entry point returns the process exit code via the C ABI".into(),
        ],
    }
}

fn multiple_main_diag(cx: &CompilationContext<'_>, entity: Entity) -> AnalyzeDiagnostic {
    let name = util::entity_name(cx.query, entity);
    AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[2].id,
        severity: DESCRIPTORS[2].default_severity,
        message: "more than one `@main` in this build".into(),
        labels: vec![DiagLabel {
            span: util::entity_span(cx.query, entity),
            message: format!("'{name}' is also marked `@main`"),
            is_primary: true,
        }],
        notes: vec!["an executable build must have exactly one `@main`".into()],
    }
}

fn missing_main_diag(cx: &CompilationContext<'_>) -> AnalyzeDiagnostic {
    AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[3].id,
        severity: DESCRIPTORS[3].default_severity,
        message: "no entry point: an executable build requires a `@main` function".into(),
        labels: vec![DiagLabel {
            span: entry_module_span(cx),
            message: "no `@main` function found in this build".into(),
            is_primary: true,
        }],
        notes: vec!["mark the entry-point function with `@main`".into()],
    }
}
