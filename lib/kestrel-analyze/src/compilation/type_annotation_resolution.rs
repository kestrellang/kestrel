//! # Type Annotation Resolution Analyzer
//!
//! Validates that all type references within `TypeAnnotation` components resolve
//! to known types. Walks the entity tree and for each entity with a
//! `TypeAnnotation`, recursively checks that every named type in the AST type
//! can be resolved via `ResolveTypePath`.
//!
//! This is a uniform check across all entity kinds (fields, type aliases,
//! function return types, parameters, etc.) — anywhere a `TypeAnnotation`
//! component exists.
//!
//! ## Diagnostics
//!
//! ### E436 -- `unresolved_type_in_annotation` (Error, Correctness)
//! **Message:** "cannot find type '{name}' in this scope"

use crate::context::CompilationContext;
use crate::diagnostic::*;
use crate::traits::{CompilationCheck, Describe};
use kestrel_ast::AstType;
use kestrel_ast_builder::{NodeKind, TypeAnnotation};
use kestrel_hecs::Entity;
use kestrel_name_res::{ResolveTypePath, TypeResolution};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E436",
    name: "unresolved_type_in_annotation",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct TypeAnnotationResolutionAnalyzer;

impl Describe for TypeAnnotationResolutionAnalyzer {
    fn id(&self) -> &'static str {
        "type_annotation_resolution"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl CompilationCheck for TypeAnnotationResolutionAnalyzer {
    fn check(&self, cx: &CompilationContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();
        walk_entity(cx, cx.root, false, &mut diags);
        diags
    }
}

/// Recursively walk the entity tree checking TypeAnnotation components.
/// `in_protocol` tracks whether we're inside a protocol declaration.
/// Inside protocols, only type alias defaults are checked — function/subscript
/// signatures reference abstract types (Self, associated types) validated elsewhere.
/// Skipping them also avoids infinite recursion on cyclic protocol inheritance.
fn walk_entity(
    cx: &CompilationContext<'_>,
    entity: Entity,
    in_protocol: bool,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let kind = cx.query.get::<NodeKind>(entity);
    let is_protocol = kind == Some(&NodeKind::Protocol);

    // Decide whether to check this entity's TypeAnnotation
    let should_check = if in_protocol {
        // Inside protocols, only check type alias defaults (concrete types that must resolve)
        kind == Some(&NodeKind::TypeAlias)
    } else {
        !is_protocol
    };

    if should_check
        && let Some(ann) = cx.query.get::<TypeAnnotation>(entity) {
            // Resolve context: use the entity itself so its own type params are in scope.
            // ResolveName walks up the hierarchy, so parent/ancestor names are also found.
            check_ast_type(cx, &ann.0, entity, diags);
        }

    for &child in cx.query.children_of(entity) {
        walk_entity(cx, child, in_protocol || is_protocol, diags);
    }
}

/// Recursively check that all named types in an AstType resolve.
fn check_ast_type(
    cx: &CompilationContext<'_>,
    ast_ty: &AstType,
    context: Entity,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    match ast_ty {
        AstType::Named { segments, span } => {
            // Resolve the base type path (segment names without type args)
            let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();

            // Skip `Self` — it's resolved contextually, not via ResolveTypePath
            if seg_names.len() == 1 && seg_names[0] == "Self" {
                return;
            }

            let resolution = cx.query.query(ResolveTypePath {
                segments: seg_names,
                context,
                root: cx.root,
            });

            if let TypeResolution::NotFound(name) = resolution {
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message: format!("cannot find type '{}' in this scope", name),
                    labels: vec![DiagLabel {
                        span: span.clone(),
                        message: "not found".into(),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }

            // Also check type arguments recursively
            for seg in segments {
                for arg in &seg.type_args {
                    check_ast_type(cx, arg, context, diags);
                }
            }
        },
        AstType::Tuple(types, _) => {
            for ty in types {
                check_ast_type(cx, ty, context, diags);
            }
        },
        AstType::Function {
            params,
            return_type,
            ..
        } => {
            for p in params {
                check_ast_type(cx, p, context, diags);
            }
            check_ast_type(cx, return_type, context, diags);
        },
        AstType::Array(inner, _) | AstType::Optional(inner, _) => {
            check_ast_type(cx, inner, context, diags);
        },
        AstType::Dictionary(key, value, _) => {
            check_ast_type(cx, key, context, diags);
            check_ast_type(cx, value, context, diags);
        },
        AstType::Result { ok, err, .. } => {
            check_ast_type(cx, ok, context, diags);
            check_ast_type(cx, err, context, diags);
        },
        // Unit, Never, Inferred — no type references to check
        AstType::Unit(_) | AstType::Never(_) | AstType::Inferred(_) => {},
    }
}
