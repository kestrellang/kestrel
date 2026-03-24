//! # Generics Analyzer
//!
//! Validates generic type parameter declarations and where clause bounds:
//!
//! 1. **Duplicate type parameter names** — Two parameters with the same name
//!    within a single generic declaration.
//! 2. **Default ordering** — Parameters with defaults must come after those
//!    without. A non-default parameter after a defaulted one is an error.
//! 3. **Where clause bounds must be protocols** — Not yet checkable because
//!    bound types are unresolved AstTypes. Shell for now.
//!
//! ## Diagnostics
//!
//! ### E434 -- `duplicate_type_parameter` (Error, Correctness)
//!
//! **Message:** "duplicate type parameter name '{name}'"
//!
//! **Labels:**
//! - Primary: the duplicate type parameter
//!   - Span source: `util::entity_span` on the second TypeParameter child entity
//!   - Message: "duplicate type parameter"
//! - Secondary: the first type parameter with the same name
//!   - Span source: `util::entity_span` on the first TypeParameter child entity
//!   - Message: "first defined here"
//!
//! **Notes:** (none)
//!
//! ### E435 -- `type_parameter_default_ordering` (Error, Correctness)
//!
//! **Message:** "type parameter '{without}' without default follows '{with_default}' which has a default"
//!
//! **Labels:**
//! - Primary: the non-default parameter after a defaulted one
//!   - Span source: `util::entity_span` on the non-default TypeParameter entity
//!   - Message: "parameter without default"
//! - Secondary: the first parameter with a default
//!   - Span source: `util::entity_span` on the defaulted TypeParameter entity
//!   - Message: "parameter with default"
//!
//! **Notes:**
//! - "type parameters with defaults must come after those without"
//!
//! ### E436 -- `non_protocol_bound` (Error, Correctness)
//!
//! **Message:** "bound '{type_name}' is a {type_kind}, not a protocol"
//!
//! **Labels:**
//! - Primary: the where clause bound
//!   - Span source: the bound's syntax node span
//!   - Message: "expected a protocol"
//!
//! **Notes:**
//! - "only protocols can be used as type bounds in where clauses"
//!
//! ### E437 -- `undeclared_type_parameter_in_where` (Error, Correctness)
//!
//! **Message:** "undeclared type parameter '{name}' in where clause"
//!
//! **Labels:**
//! - Primary: the undeclared type parameter reference
//!   - Span source: the subject's syntax node span
//!   - Message: "not a declared type parameter"
//!
//! **Notes:**
//! - "available type parameters: {list}"

use std::collections::HashMap;

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast::AstType;
use kestrel_ast_builder::{Name, NodeKind, TypeAnnotation, TypeParams, WhereClause as AstWhereClause, WhereConstraint};
use kestrel_name_res::{ResolveTypePath, TypeResolution};
use kestrel_span2::Span;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E434",
        name: "duplicate_type_parameter",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E435",
        name: "type_parameter_default_ordering",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E436",
        name: "non_protocol_bound",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E437",
        name: "undeclared_type_parameter_in_where",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E438",
        name: "type_arg_arity",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E439",
        name: "type_param_shadows_outer",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct GenericsAnalyzer;

impl Describe for GenericsAnalyzer {
    fn id(&self) -> &'static str {
        "generics"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for GenericsAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[
            NodeKind::Function,
            NodeKind::Struct,
            NodeKind::Enum,
            NodeKind::Protocol,
            NodeKind::TypeAlias,
            NodeKind::Initializer,
        ]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let Some(type_params) = cx.query.get::<TypeParams>(cx.entity) else {
            return vec![];
        };
        if type_params.0.is_empty() {
            return vec![];
        }

        let mut diags = Vec::new();

        check_duplicate_type_params(cx, &type_params.0, &mut diags);
        check_default_ordering(cx, &type_params.0, &mut diags);

        check_where_clause_bounds(cx, &type_params.0, &mut diags);
        check_type_param_shadowing(cx, &type_params.0, &mut diags);

        diags
    }
}

/// Check for duplicate type parameter names (E434).
fn check_duplicate_type_params(
    cx: &DeclContext<'_>,
    params: &[kestrel_hecs::Entity],
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let mut seen: HashMap<String, Span> = HashMap::new();

    for &param_entity in params {
        let name = util::entity_name(cx.query, param_entity);
        let span = util::entity_span(cx.query, param_entity);

        if let Some(first_span) = seen.get(&name) {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[0].id,
                severity: DESCRIPTORS[0].default_severity,
                message: format!("duplicate type parameter name '{}'", name),
                labels: vec![
                    DiagLabel {
                        span,
                        message: "duplicate type parameter".into(),
                        is_primary: true,
                    },
                    DiagLabel {
                        span: first_span.clone(),
                        message: "first defined here".into(),
                        is_primary: false,
                    },
                ],
                notes: vec![],
            });
        } else {
            seen.insert(name, span);
        }
    }
}

/// Analyzer: validates type argument arity at type usage sites (e.g., `type Bad = Map[Int]`).
/// Targets TypeAlias entities and checks their TypeAnnotation for correct arity.
pub struct TypeArgArityAnalyzer;

impl Describe for TypeArgArityAnalyzer {
    fn id(&self) -> &'static str {
        "type_arg_arity"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        &DESCRIPTORS[4..5] // E438
    }
}

impl DeclCheck for TypeArgArityAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::TypeAlias]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let Some(ann) = cx.query.get::<TypeAnnotation>(cx.entity) else {
            return vec![];
        };
        let mut diags = Vec::new();
        check_type_arg_arity(cx, &ann.0, &mut diags);
        diags
    }
}

/// Recursively check type argument arity in an AstType.
fn check_type_arg_arity(
    cx: &DeclContext<'_>,
    ty: &AstType,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    match ty {
        AstType::Named { segments, span } => {
            // Collect all type args from segments
            let type_args: Vec<&AstType> = segments.iter()
                .flat_map(|s| s.type_args.iter())
                .collect();

            if !type_args.is_empty() {
                // Resolve the named type to check its type params
                let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
                let result = cx.query.query(ResolveTypePath {
                    segments: seg_names,
                    context: cx.entity,
                    root: cx.root,
                });
                if let TypeResolution::Found(entity) = result {
                    let total = cx.query.get::<TypeParams>(entity)
                        .map(|tp| tp.0.len())
                        .unwrap_or(0);
                    let required = cx.query.get::<TypeParams>(entity)
                        .map(|tp| tp.0.iter()
                            .filter(|&&p| cx.query.get::<TypeAnnotation>(p).is_none())
                            .count())
                        .unwrap_or(0);
                    let type_name = segments.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(".");

                    if total == 0 {
                        // Non-generic type doesn't accept type arguments
                        diags.push(AnalyzeDiagnostic {
                            descriptor_id: "E438",
                            severity: Severity::Error,
                            message: format!(
                                "'{}' does not accept type arguments",
                                type_name
                            ),
                            labels: vec![DiagLabel {
                                span: span.clone(),
                                message: "does not accept type arguments".into(),
                                is_primary: true,
                            }],
                            notes: vec![],
                        });
                    } else if type_args.len() < required {
                        diags.push(AnalyzeDiagnostic {
                            descriptor_id: "E438",
                            severity: Severity::Error,
                            message: format!(
                                "too few type arguments for '{}': expected at least {}, got {}",
                                type_name, required, type_args.len()
                            ),
                            labels: vec![DiagLabel {
                                span: span.clone(),
                                message: format!("expected at least {} type argument(s)", required),
                                is_primary: true,
                            }],
                            notes: vec![],
                        });
                    } else if type_args.len() > total {
                        diags.push(AnalyzeDiagnostic {
                            descriptor_id: "E438",
                            severity: Severity::Error,
                            message: format!(
                                "too many type arguments for '{}': expected at most {}, got {}",
                                type_name, total, type_args.len()
                            ),
                            labels: vec![DiagLabel {
                                span: span.clone(),
                                message: format!("expected at most {} type argument(s)", total),
                                is_primary: true,
                            }],
                            notes: vec![],
                        });
                    }
                }
            }

            // Recurse into type arguments
            for seg in segments {
                for arg in &seg.type_args {
                    check_type_arg_arity(cx, arg, diags);
                }
            }
        }
        AstType::Tuple(types, _) => {
            for t in types { check_type_arg_arity(cx, t, diags); }
        }
        AstType::Function { params, return_type, .. } => {
            for p in params { check_type_arg_arity(cx, p, diags); }
            check_type_arg_arity(cx, return_type, diags);
        }
        AstType::Array(inner, _) | AstType::Optional(inner, _) => {
            check_type_arg_arity(cx, inner, diags);
        }
        AstType::Dictionary(k, v, _) | AstType::Result { ok: k, err: v, .. } => {
            check_type_arg_arity(cx, k, diags);
            check_type_arg_arity(cx, v, diags);
        }
        _ => {}
    }
}

/// Check that type parameters with defaults come after those without (E435).
/// A TypeParameter has a default if it has a TypeAnnotation component.
fn check_default_ordering(
    cx: &DeclContext<'_>,
    params: &[kestrel_hecs::Entity],
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    // Track the first parameter that has a default
    let mut first_with_default: Option<(String, Span)> = None;

    for &param_entity in params {
        let has_default = cx.query.get::<TypeAnnotation>(param_entity).is_some();

        if has_default {
            if first_with_default.is_none() {
                let name = util::entity_name(cx.query, param_entity);
                let span = util::entity_span(cx.query, param_entity);
                first_with_default = Some((name, span));
            }
        } else if let Some((ref default_name, ref default_span)) = first_with_default {
            // Non-default parameter after one with a default — error
            let name = util::entity_name(cx.query, param_entity);
            let span = util::entity_span(cx.query, param_entity);
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[1].id,
                severity: DESCRIPTORS[1].default_severity,
                message: format!(
                    "type parameter '{}' without default follows '{}' which has a default",
                    name, default_name
                ),
                labels: vec![
                    DiagLabel {
                        span,
                        message: "parameter without default".into(),
                        is_primary: true,
                    },
                    DiagLabel {
                        span: default_span.clone(),
                        message: "parameter with default".into(),
                        is_primary: false,
                    },
                ],
                notes: vec!["type parameters with defaults must come after those without".into()],
            });
            // One diagnostic is enough — stop checking
            break;
        }
    }
}

/// Check that type parameters don't shadow outer scope type parameters (E439).
/// E.g., `struct Box[T] { func identity[T](...) }` — inner T shadows outer T.
fn check_type_param_shadowing(
    cx: &DeclContext<'_>,
    params: &[kestrel_hecs::Entity],
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    // Collect outer type param names by walking parent chain
    let mut outer_params: HashMap<String, Span> = HashMap::new();
    let mut current = cx.query.parent_of(cx.entity);
    while let Some(ancestor) = current {
        if let Some(tp) = cx.query.get::<TypeParams>(ancestor) {
            for &p in &tp.0 {
                let name = util::entity_name(cx.query, p);
                let span = util::entity_span(cx.query, p);
                outer_params.entry(name).or_insert(span);
            }
        }
        current = cx.query.parent_of(ancestor);
    }

    if outer_params.is_empty() {
        return;
    }

    for &param_entity in params {
        let name = util::entity_name(cx.query, param_entity);
        if let Some(outer_span) = outer_params.get(&name) {
            let span = util::entity_span(cx.query, param_entity);
            diags.push(AnalyzeDiagnostic {
                descriptor_id: "E439",
                severity: Severity::Error,
                message: format!("type parameter '{}' shadows outer type parameter", name),
                labels: vec![
                    DiagLabel {
                        span,
                        message: format!("'{}' shadows outer type parameter", name),
                        is_primary: true,
                    },
                    DiagLabel {
                        span: outer_span.clone(),
                        message: "outer type parameter defined here".into(),
                        is_primary: false,
                    },
                ],
                notes: vec![],
            });
        }
    }
}

/// Validate where clause bounds:
/// - E436: bound is not a protocol (e.g., `T: SomeStruct`)
/// - E437: subject is not a declared type parameter (e.g., `U: Protocol` when only `T` is declared)
/// Also catches unresolved bounds (e.g., `T: NonExistent`).
fn check_where_clause_bounds(
    cx: &DeclContext<'_>,
    params: &[kestrel_hecs::Entity],
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let Some(wc) = cx.query.get::<AstWhereClause>(cx.entity) else {
        return;
    };

    // Collect declared type param names for checking subjects
    let declared_names: Vec<String> = params.iter()
        .filter_map(|&p| cx.query.get::<Name>(p).map(|n| n.0.clone()))
        .collect();

    for constraint in &wc.0 {
        let (subject, protocols) = match constraint {
            WhereConstraint::Bound { subject, protocols, .. } => (subject, protocols.as_slice()),
            WhereConstraint::NegativeBound { subject, protocol, .. } => (subject, std::slice::from_ref(protocol)),
            WhereConstraint::Equality { .. } => continue,
        };

        // Check subject is a declared type parameter (E437)
        if let AstType::Named { segments, span } = subject {
            if segments.len() == 1 {
                let name = &segments[0].name;
                if !declared_names.contains(name) {
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: "E437",
                        severity: Severity::Error,
                        message: format!("undeclared type parameter '{}' in where clause", name),
                        labels: vec![DiagLabel {
                            span: span.clone(),
                            message: "not a declared type parameter".into(),
                            is_primary: true,
                        }],
                        notes: vec![format!("available type parameters: {}", declared_names.join(", "))],
                    });
                }
            }
        }

        // Check each bound resolves to a protocol (E436) or exists at all
        for proto_ty in protocols {
            let AstType::Named { segments, span } = proto_ty else { continue };
            let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
            let type_name = seg_names.join(".");

            let result = cx.query.query(ResolveTypePath {
                segments: seg_names,
                context: cx.entity,
                root: cx.root,
            });

            match result {
                TypeResolution::Found(entity) => {
                    let kind = cx.query.get::<NodeKind>(entity);
                    if kind != Some(&NodeKind::Protocol) {
                        let kind_str = kind
                            .map(|k| format!("{k:?}").to_lowercase())
                            .unwrap_or_else(|| "type".into());
                        diags.push(AnalyzeDiagnostic {
                            descriptor_id: "E436",
                            severity: Severity::Error,
                            message: format!("'{}' is not a protocol", type_name),
                            labels: vec![DiagLabel {
                                span: span.clone(),
                                message: format!("'{}' is a {}, not a protocol", type_name, kind_str),
                                is_primary: true,
                            }],
                            notes: vec!["only protocols can be used as type bounds in where clauses".into()],
                        });
                    }
                }
                TypeResolution::NotFound(_) => {
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: "E436",
                        severity: Severity::Error,
                        message: format!("cannot find type '{}' in this scope", type_name),
                        labels: vec![DiagLabel {
                            span: span.clone(),
                            message: format!("not found"),
                            is_primary: true,
                        }],
                        notes: vec![],
                    });
                }
                _ => {}
            }
        }
    }
}
