//! `callHierarchy/prepare`, `callHierarchy/incomingCalls`,
//! `callHierarchy/outgoingCalls`.
//!
//! Prepare resolves the callable at the cursor into a `CallHierarchyItem`.
//! Incoming calls use `references_to` filtered to call expressions.
//! Outgoing calls walk the body's HIR for Call/MethodCall/ProtocolCall.

use std::collections::HashMap;

use kestrel_ast_builder::{Callable, DeclSpan, FileId, FilePath, Name, NodeKind};
use kestrel_hecs::{Entity, World};
use kestrel_hir::body::HirExpr;
use kestrel_hir_lower::LowerBody;
use kestrel_type_infer::InferBody;
use tower_lsp::lsp_types::{
    CallHierarchyIncomingCall, CallHierarchyIncomingCallsParams, CallHierarchyItem,
    CallHierarchyOutgoingCall, CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams, Range,
    SymbolKind,
};

use crate::handlers::document_symbols::node_kind_to_symbol_kind;
use crate::position::LineIndex;
use crate::references::{self, entity_file};
use crate::semantic;
use crate::server::{SharedState, path_to_url, url_to_path};

// ===== prepare =====

pub async fn prepare(
    state: SharedState,
    params: CallHierarchyPrepareParams,
) -> Option<Vec<CallHierarchyItem>> {
    let uri = params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let path = url_to_path(&uri);

    let (handle, stdlib, user, sources, line_index) = {
        let s = state.lock().await;
        let li = s.docs.get(&uri).map(|d| d.line_index.clone())?;
        let (stdlib, user) = s.partition_sources();
        (
            s.compiler_handle.clone(),
            stdlib,
            user,
            s.sources.clone(),
            li,
        )
    };
    let offset = line_index.position_to_offset(pos);

    handle
        .with_compiler(
            stdlib,
            user,
            move |compiler, _by_path| -> Option<Vec<CallHierarchyItem>> {
                let file_entity = semantic::file_entity_for_path(compiler, &path)?;
                let world = compiler.world();
                let root = compiler.root();

                // Find the entity at cursor.
                let entity = resolve_callable_at(world, file_entity, offset, root)?;
                let item = entity_to_item(world, &sources, entity)?;
                Some(vec![item])
            },
        )
        .await
        .flatten()
}

// ===== incoming calls =====

pub async fn incoming(
    state: SharedState,
    params: CallHierarchyIncomingCallsParams,
) -> Option<Vec<CallHierarchyIncomingCall>> {
    let item_uri = params.item.uri.clone();
    let item_range = params.item.selection_range;

    let (handle, stdlib, user, sources) = {
        let s = state.lock().await;
        let (stdlib, user) = s.partition_sources();
        (s.compiler_handle.clone(), stdlib, user, s.sources.clone())
    };
    let item_path = url_to_path(&item_uri);

    handle
        .with_compiler(
            stdlib,
            user,
            move |compiler, _by_path| -> Option<Vec<CallHierarchyIncomingCall>> {
                let file_entity = semantic::file_entity_for_path(compiler, &item_path)?;
                let world = compiler.world();
                let root = compiler.root();

                let li = LineIndex::new(sources.get(&item_path)?.clone());
                let offset = li.position_to_offset(item_range.start);
                let target = resolve_callable_at(world, file_entity, offset, root)?;

                let refs = references::references_to(world, root, target);

                // Group call sites by their enclosing callable.
                let mut by_caller: HashMap<Entity, Vec<Range>> = HashMap::new();
                for site in &refs {
                    let Some(body) = enclosing_callable(world, site.file, site.span.start) else {
                        continue;
                    };
                    let Some(source) = file_source(world, site.file, &sources) else {
                        continue;
                    };
                    let site_li = LineIndex::new(source.clone());
                    let clipped = references::clip_to_identifier(source, &site.span, site.kind);
                    let range = site_li.range_for(clipped.start, clipped.end);
                    by_caller.entry(body).or_default().push(range);
                }

                let mut results = Vec::new();
                for (caller, ranges) in by_caller {
                    if let Some(item) = entity_to_item(world, &sources, caller) {
                        results.push(CallHierarchyIncomingCall {
                            from: item,
                            from_ranges: ranges,
                        });
                    }
                }
                if results.is_empty() {
                    None
                } else {
                    Some(results)
                }
            },
        )
        .await
        .flatten()
}

// ===== outgoing calls =====

pub async fn outgoing(
    state: SharedState,
    params: CallHierarchyOutgoingCallsParams,
) -> Option<Vec<CallHierarchyOutgoingCall>> {
    let item_uri = params.item.uri.clone();
    let item_range = params.item.selection_range;

    let (handle, stdlib, user, sources) = {
        let s = state.lock().await;
        let (stdlib, user) = s.partition_sources();
        (s.compiler_handle.clone(), stdlib, user, s.sources.clone())
    };
    let item_path = url_to_path(&item_uri);

    handle
        .with_compiler(
            stdlib,
            user,
            move |compiler, _by_path| -> Option<Vec<CallHierarchyOutgoingCall>> {
                let file_entity = semantic::file_entity_for_path(compiler, &item_path)?;
                let world = compiler.world();
                let root = compiler.root();

                let li = LineIndex::new(sources.get(&item_path)?.clone());
                let offset = li.position_to_offset(item_range.start);
                let body_entity = resolve_callable_at(world, file_entity, offset, root)?;

                let ctx = world.query_context();
                let hir = ctx.query(LowerBody {
                    entity: body_entity,
                    root,
                })?;
                let typed = ctx.query(InferBody {
                    entity: body_entity,
                    root,
                });

                let source = sources.get(&item_path)?;

                // Walk body expressions, collect calls grouped by callee entity.
                let mut by_callee: HashMap<Entity, Vec<Range>> = HashMap::new();
                for (id, expr) in hir.exprs.iter() {
                    let callee = match expr {
                        HirExpr::Def(entity, _, _) => {
                            if world.get::<Callable>(*entity).is_some()
                                || matches!(
                                    world.get::<NodeKind>(*entity),
                                    Some(NodeKind::Function | NodeKind::Initializer)
                                )
                            {
                                Some(*entity)
                            } else {
                                None
                            }
                        },
                        HirExpr::Call { .. }
                        | HirExpr::MethodCall { .. }
                        | HirExpr::ProtocolCall { .. } => {
                            typed.as_ref().and_then(|t| t.resolutions.get(&id).copied())
                        },
                        _ => None,
                    };
                    if let Some(callee_entity) = callee {
                        let span = semantic::hir_expr_span(expr);
                        let clipped_span = references::clip_to_identifier(
                            source,
                            &span,
                            references::RefKind::MemberAccess,
                        );
                        let range = li.range_for(clipped_span.start, clipped_span.end);
                        by_callee.entry(callee_entity).or_default().push(range);
                    }
                }

                let mut results = Vec::new();
                for (callee, ranges) in by_callee {
                    if let Some(item) = entity_to_item(world, &sources, callee) {
                        results.push(CallHierarchyOutgoingCall {
                            to: item,
                            from_ranges: ranges,
                        });
                    }
                }
                if results.is_empty() {
                    None
                } else {
                    Some(results)
                }
            },
        )
        .await
        .flatten()
}

// ===== helpers =====

/// Resolve the callable entity at the cursor. First tries body expression
/// resolution (Def), then falls back to the enclosing declaration.
fn resolve_callable_at(
    world: &World,
    file_entity: Entity,
    offset: usize,
    root: Entity,
) -> Option<Entity> {
    if let Some(body_entity) = semantic::body_entity_at(world, file_entity, offset) {
        let ctx = world.query_context();
        if let Some(hir) = ctx.query(LowerBody {
            entity: body_entity,
            root,
        })
            && let Some(expr_id) = semantic::hir_expr_at(&hir, offset) {
                match &hir.exprs[expr_id] {
                    HirExpr::Def(entity, _, _) => {
                        if is_callable(world, *entity) {
                            return Some(*entity);
                        }
                    },
                    HirExpr::MethodCall { .. }
                    | HirExpr::Call { .. }
                    | HirExpr::ProtocolCall { .. } => {
                        let typed = ctx.query(InferBody {
                            entity: body_entity,
                            root,
                        });
                        if let Some(typed) = typed
                            && let Some(&resolved) = typed.resolutions.get(&expr_id)
                                && is_callable(world, resolved) {
                                    return Some(resolved);
                                }
                    },
                    _ => {},
                }
            }
    }
    // Fallback: cursor on a declaration name.
    let decl = semantic::enclosing_decl_at(world, file_entity, offset)?;
    if is_callable(world, decl) {
        Some(decl)
    } else {
        None
    }
}

fn is_callable(world: &World, entity: Entity) -> bool {
    matches!(
        world.get::<NodeKind>(entity),
        Some(NodeKind::Function | NodeKind::Initializer | NodeKind::Subscript | NodeKind::Deinit)
    ) || world.get::<Callable>(entity).is_some()
}

/// Find the smallest callable entity whose body spans contain `offset`.
fn enclosing_callable(world: &World, file_entity: Entity, offset: usize) -> Option<Entity> {
    let mut best: Option<(Entity, usize)> = None;
    for (entity, span) in world.iter_component::<DeclSpan>() {
        let Some(fid) = world.get::<FileId>(entity) else {
            continue;
        };
        if fid.0 != file_entity {
            continue;
        }
        if !is_callable(world, entity) {
            continue;
        }
        let s = &span.0;
        if s.start <= offset && offset <= s.end {
            let len = s.end - s.start;
            if best.map(|(_, l)| len < l).unwrap_or(true) {
                best = Some((entity, len));
            }
        }
    }
    best.map(|(e, _)| e)
}

fn entity_to_item(
    world: &World,
    sources: &HashMap<String, String>,
    entity: Entity,
) -> Option<CallHierarchyItem> {
    let name = world.get::<Name>(entity)?.0.clone();
    let kind = world
        .get::<NodeKind>(entity)
        .and_then(node_kind_to_symbol_kind)
        .unwrap_or(SymbolKind::FUNCTION);
    let file = entity_file(world, entity)?;
    let file_path = world.get::<FilePath>(file)?.0.clone();
    let url = path_to_url(&file_path)?;
    let source = sources.get(&file_path)?;
    let li = LineIndex::new(source.clone());

    let decl_span = world.get::<DeclSpan>(entity)?.0.clone();
    let range = li.range_for(decl_span.start, decl_span.end);

    let selection_range = world
        .get::<kestrel_ast_builder::CstNode>(entity)
        .and_then(|cst| kestrel_syntax_tree::utils::get_name_span(&cst.0, decl_span.file_id))
        .map(|s| li.range_for(s.start, s.end))
        .unwrap_or(range);

    Some(CallHierarchyItem {
        name,
        kind,
        tags: None,
        detail: None,
        uri: url,
        range,
        selection_range,
        data: None,
    })
}

fn file_source<'a>(
    world: &World,
    file_entity: Entity,
    sources: &'a HashMap<String, String>,
) -> Option<&'a String> {
    let path = world.get::<FilePath>(file_entity)?.0.clone();
    sources.get(&path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_compiler::Compiler;

    #[test]
    fn prepare_finds_function_at_cursor() {
        let src = "module Test\nfunc foo() -> lang.i64 { 1 }\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/ch.ks", src.into());
        c.build(f);
        let offset = src.find("foo").unwrap();
        let entity = resolve_callable_at(c.world(), f, offset, c.root());
        assert!(entity.is_some(), "expected callable at cursor");
    }

    #[test]
    fn prepare_returns_none_for_struct() {
        let src = "module Test\nstruct Point { var x: lang.i64 }\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/ch2.ks", src.into());
        c.build(f);
        let offset = src.find("Point").unwrap();
        let entity = resolve_callable_at(c.world(), f, offset, c.root());
        assert!(entity.is_none(), "struct should not be callable");
    }
}
