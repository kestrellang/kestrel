//! Parameter extraction from CST nodes.
//!
//! CST structure for a Parameter:
//! ```text
//! Parameter "x: Int64"
//!   Pattern "x"
//!     BindingPattern "x"
//!       Identifier "x"
//!   Colon ":"
//!   Ty " Int64"
//!     TyPath ...
//! ```
//! For two-name params like `with x: Int`, there may be an additional
//! label identifier before the Pattern node.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::utils::find_child;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use super::helpers::is_type_kind;

use crate::ast_type::ast_type_from_cst;
use crate::components::{
    AstParam, Body, FileId, NodeKind, ParamPattern, StructPatternField, TypeAnnotation,
};
use crate::lower;

/// Extract parameters from a node containing a ParameterList child.
/// Creates child entities for default value expressions.
pub fn extract_params(
    world: &mut World,
    node: &SyntaxNode,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) -> Vec<AstParam> {
    let param_list = match find_child(node, SyntaxKind::ParameterList) {
        Some(list) => list,
        None => return Vec::new(),
    };

    let params: Vec<AstParam> = param_list
        .children()
        .filter(|c| c.kind() == SyntaxKind::Parameter)
        .filter_map(|param_node| {
            extract_single_param(world, &param_node, parent, file_entity, file_id)
        })
        .collect();

    // Detect defaults that reference sibling params. Replace the body with an
    // empty one (suppresses "undefined name" from inference) and set a marker
    // component so the analyzer can emit a proper diagnostic.
    let param_names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
    for param in &params {
        let Some(default_entity) = param.default_entity else {
            continue;
        };
        let Some(body) = world.get::<crate::components::Body>(default_entity) else {
            continue;
        };
        if let Some(referenced) = default_body_references_param(&body.0, &param_names) {
            world.set(
                default_entity,
                crate::components::DefaultReferencesParam(referenced.to_string()),
            );
            // Replace with empty body so inference doesn't produce "undefined name"
            world.set(
                default_entity,
                crate::components::Body(kestrel_ast::ast_body::AstBody {
                    exprs: kestrel_ast::arena::Arena::new(),
                    pats: kestrel_ast::arena::Arena::new(),
                    stmts: kestrel_ast::arena::Arena::new(),
                    statements: Vec::new(),
                    tail_expr: None,
                }),
            );
        }
    }

    params
}

/// Extract a single parameter from a Parameter CST node.
///
/// The bind name comes from Pattern > BindingPattern > Identifier.
/// A label (if any) is a bare Identifier token at the top level before
/// the Pattern node.
/// Counter for generating synthetic parameter names (_0, _1, ...).
/// Reset per parameter list via `extract_params`.
static PARAM_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

fn extract_single_param(
    world: &mut World,
    node: &SyntaxNode,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) -> Option<AstParam> {
    // Check for mutating/consuming access mode
    let is_consuming = node.children_with_tokens().any(|e| {
        e.as_token()
            .is_some_and(|t| t.kind() == SyntaxKind::Consuming)
    });
    let is_mut = is_consuming
        || node.children_with_tokens().any(|e| {
            e.as_token()
                .is_some_and(|t| t.kind() == SyntaxKind::Mutating)
        });

    let pattern_node = find_child(node, SyntaxKind::Pattern)?;

    // Try simple binding pattern first: Pattern > BindingPattern > Identifier
    let simple_name = find_child(&pattern_node, SyntaxKind::BindingPattern).and_then(|bp| {
        bp.children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::Identifier)
            .map(|t| t.text().to_string())
    });

    // For destructured patterns (tuple, struct, wildcard), extract the pattern
    // and generate a synthetic name
    let (name, pattern) = if let Some(name) = simple_name {
        (name, None)
    } else {
        let param_pat = extract_param_pattern(&pattern_node);
        param_pat.as_ref()?;
        let idx = PARAM_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        (format!("_param_{}", idx), param_pat)
    };

    // Check for a label before the Pattern child. The emitter wraps labels
    // in Name nodes (Name > Identifier), but they could also appear as bare
    // Identifier tokens. Handle both forms.
    let mut label = None;
    for elem in node.children_with_tokens() {
        match elem {
            rowan::NodeOrToken::Node(n) if n.kind() == SyntaxKind::Pattern => break,
            rowan::NodeOrToken::Node(n) if n.kind() == SyntaxKind::Name => {
                // Label wrapped in Name node: Name > Identifier
                label = n
                    .children_with_tokens()
                    .filter_map(|e| e.into_token())
                    .find(|t| t.kind() == SyntaxKind::Identifier)
                    .map(|t| t.text().to_string());
            },
            rowan::NodeOrToken::Token(t) if t.kind() == SyntaxKind::Identifier => {
                label = Some(t.text().to_string());
            },
            rowan::NodeOrToken::Token(t) if t.kind() == SyntaxKind::Underscore => {
                // Explicit no-label marker — leave label as None
                label = None;
            },
            _ => {},
        }
    }

    // Extract type annotation
    let ty = node
        .children()
        .find(|c| is_type_kind(c.kind()))
        .and_then(|c| ast_type_from_cst(&c, file_id));

    // Create child entity for default value expression
    let default_entity = find_child(node, SyntaxKind::DefaultValue).map(|default_node| {
        let entity = world.spawn();
        world.set(entity, NodeKind::ParamDefault);
        world.set(entity, FileId(file_entity));
        world.set(
            entity,
            Body(lower::lower_default_value(&default_node, file_id)),
        );
        // Store the param's type annotation so inference checks the default against it
        if let Some(ref param_ty) = ty {
            world.set(entity, TypeAnnotation(param_ty.clone()));
        }
        world.set_parent(entity, parent);
        entity
    });

    Some(AstParam {
        label,
        name,
        ty,
        is_consuming,
        default_entity,
        pattern,
        is_mut,
    })
}

/// Extract a ParamPattern from a Pattern CST node.
/// Handles tuple patterns, struct patterns, wildcard, and binding.
fn extract_param_pattern(node: &SyntaxNode) -> Option<ParamPattern> {
    // Unwrap Pattern wrapper if present
    let inner = if node.kind() == SyntaxKind::Pattern {
        node.children().next()?
    } else {
        node.clone()
    };

    match inner.kind() {
        SyntaxKind::WildcardPattern => Some(ParamPattern::Wildcard),

        SyntaxKind::BindingPattern => {
            let is_mut = inner
                .children_with_tokens()
                .any(|e| e.as_token().is_some_and(|t| t.kind() == SyntaxKind::Var));
            let name = inner
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .find(|t| t.kind() == SyntaxKind::Identifier)
                .map(|t| t.text().to_string())
                .unwrap_or_default();
            Some(ParamPattern::Binding { name, is_mut })
        },

        SyntaxKind::TuplePattern => {
            let elements: Vec<ParamPattern> = inner
                .children()
                .filter(|c| c.kind() == SyntaxKind::TuplePatternElement)
                .filter_map(|c| {
                    // TuplePatternElement contains a pattern node directly
                    // (BindingPattern, TuplePattern, etc.) or wrapped in Pattern
                    c.children()
                        .find(|cc| {
                            cc.kind() == SyntaxKind::Pattern
                                || cc.kind() == SyntaxKind::BindingPattern
                                || cc.kind() == SyntaxKind::TuplePattern
                                || cc.kind() == SyntaxKind::StructPattern
                                || cc.kind() == SyntaxKind::WildcardPattern
                        })
                        .and_then(|p| extract_param_pattern(&p))
                })
                .collect();
            Some(ParamPattern::Tuple { elements })
        },

        SyntaxKind::StructPattern => {
            // Extract struct name
            let type_name = inner
                .children()
                .find(|c| c.kind() == SyntaxKind::ExprPath || c.kind() == SyntaxKind::TyPath)
                .and_then(|path| {
                    path.children_with_tokens()
                        .filter_map(|e| e.into_token())
                        .find(|t| t.kind() == SyntaxKind::Identifier)
                        .map(|t| t.text().to_string())
                })
                .or_else(|| {
                    // Try bare identifier
                    inner
                        .children_with_tokens()
                        .filter_map(|e| e.into_token())
                        .find(|t| t.kind() == SyntaxKind::Identifier)
                        .map(|t| t.text().to_string())
                })
                .unwrap_or_default();

            let has_rest = inner.children().any(|c| {
                c.kind() == SyntaxKind::RestPattern || c.kind() == SyntaxKind::StructPatternRest
            });

            let fields: Vec<StructPatternField> = inner
                .children()
                .filter(|c| c.kind() == SyntaxKind::StructPatternField)
                .filter_map(|field| {
                    // Field has: Identifier (field name), optional Pattern (binding)
                    let field_name = field
                        .children_with_tokens()
                        .filter_map(|e| e.into_token())
                        .find(|t| t.kind() == SyntaxKind::Identifier)
                        .map(|t| t.text().to_string())?;

                    // Check for explicit binding pattern (field: pattern)
                    // The emitter puts the binding directly (no Pattern wrapper)
                    let pattern = field
                        .children()
                        .find(|c| {
                            c.kind() == SyntaxKind::Pattern
                                || c.kind() == SyntaxKind::BindingPattern
                                || c.kind() == SyntaxKind::TuplePattern
                                || c.kind() == SyntaxKind::WildcardPattern
                                || c.kind() == SyntaxKind::StructPattern
                        })
                        .and_then(|p| extract_param_pattern(&p))
                        .unwrap_or(ParamPattern::Binding {
                            name: field_name.clone(),
                            is_mut: false,
                        });

                    Some(StructPatternField {
                        field_name,
                        pattern,
                    })
                })
                .collect();

            Some(ParamPattern::Struct {
                type_name,
                fields,
                has_rest,
            })
        },

        _ => None,
    }
}

/// Check if a default value body's tail expression is a single-segment path
/// matching a sibling parameter name. Returns the matched name if found.
fn default_body_references_param<'a>(
    body: &kestrel_ast::ast_body::AstBody,
    param_names: &[&'a str],
) -> Option<&'a str> {
    let tail_id = body.tail_expr?;
    if let kestrel_ast::ast_body::AstExpr::Path { segments, .. } = &body.exprs[tail_id]
        && segments.len() == 1 && segments[0].type_args.is_none() {
            return param_names
                .iter()
                .find(|&&p| p == segments[0].name)
                .copied();
        }
    None
}
