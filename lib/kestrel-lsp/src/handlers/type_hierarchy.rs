//! `typeHierarchy/prepare`, `typeHierarchy/supertypes`,
//! `typeHierarchy/subtypes`.
//!
//! Prepare resolves the type at the cursor into a `TypeHierarchyItem`.
//! Supertypes returns the protocols a type conforms to (via
//! `ConformingProtocols`). Subtypes performs an O(n) scan of all nominal
//! entities to find types that conform to a given protocol.

use std::collections::HashMap;

use kestrel_ast_builder::{DeclSpan, FilePath, Name, NodeKind};
use kestrel_hecs::{Entity, World};
use kestrel_hir::body::HirExpr;
use kestrel_hir_lower::LowerBody;
use kestrel_name_res::ConformingProtocols;
use kestrel_type_infer::InferBody;
use tower_lsp::lsp_types::{
    SymbolKind, TypeHierarchyItem, TypeHierarchyPrepareParams, TypeHierarchySubtypesParams,
    TypeHierarchySupertypesParams,
};

use crate::handlers::document_symbols::node_kind_to_symbol_kind;
use crate::position::LineIndex;
use crate::references::entity_file;
use crate::semantic;
use crate::server::{SharedState, path_to_url, url_to_path};

// ===== prepare =====

pub async fn prepare(
    state: SharedState,
    params: TypeHierarchyPrepareParams,
) -> Option<Vec<TypeHierarchyItem>> {
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
            move |compiler, _by_path| -> Option<Vec<TypeHierarchyItem>> {
                let file_entity = semantic::file_entity_for_path(compiler, &path)?;
                let world = compiler.world();
                let root = compiler.root();

                let entity = resolve_type_at(world, file_entity, offset, root, compiler)?;
                let item = entity_to_item(world, &sources, entity)?;
                Some(vec![item])
            },
        )
        .await
        .flatten()
}

// ===== supertypes =====

pub async fn supertypes(
    state: SharedState,
    params: TypeHierarchySupertypesParams,
) -> Option<Vec<TypeHierarchyItem>> {
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
            move |compiler, _by_path| -> Option<Vec<TypeHierarchyItem>> {
                let file_entity = semantic::file_entity_for_path(compiler, &item_path)?;
                let world = compiler.world();
                let root = compiler.root();

                let li = LineIndex::new(sources.get(&item_path)?.clone());
                let offset = li.position_to_offset(item_range.start);
                let entity = resolve_type_at(world, file_entity, offset, root, compiler)?;

                let ctx = world.query_context();
                let protocols = ctx.query(ConformingProtocols { entity, root });

                let mut items = Vec::new();
                for proto in &protocols {
                    if let Some(item) = entity_to_item(world, &sources, *proto) {
                        items.push(item);
                    }
                }
                if items.is_empty() { None } else { Some(items) }
            },
        )
        .await
        .flatten()
}

// ===== subtypes =====

pub async fn subtypes(
    state: SharedState,
    params: TypeHierarchySubtypesParams,
) -> Option<Vec<TypeHierarchyItem>> {
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
            move |compiler, _by_path| -> Option<Vec<TypeHierarchyItem>> {
                let file_entity = semantic::file_entity_for_path(compiler, &item_path)?;
                let world = compiler.world();
                let root = compiler.root();

                let li = LineIndex::new(sources.get(&item_path)?.clone());
                let offset = li.position_to_offset(item_range.start);
                let target = resolve_type_at(world, file_entity, offset, root, compiler)?;

                // O(n) scan over all nominal entities.
                let ctx = world.query_context();
                let mut items = Vec::new();
                for (entity, kind) in world.iter_component::<NodeKind>() {
                    if !matches!(kind, NodeKind::Struct | NodeKind::Enum) {
                        continue;
                    }
                    let protos = ctx.query(ConformingProtocols { entity, root });
                    if protos.contains(&target) {
                        if let Some(item) = entity_to_item(world, &sources, entity) {
                            items.push(item);
                        }
                    }
                }
                // For protocol subtypes: protocols that inherit from target.
                for (entity, kind) in world.iter_component::<NodeKind>() {
                    if *kind != NodeKind::Protocol || entity == target {
                        continue;
                    }
                    let protos = ctx.query(ConformingProtocols { entity, root });
                    if protos.contains(&target) {
                        if let Some(item) = entity_to_item(world, &sources, entity) {
                            items.push(item);
                        }
                    }
                }

                if items.is_empty() { None } else { Some(items) }
            },
        )
        .await
        .flatten()
}

// ===== helpers =====

fn resolve_type_at(
    world: &World,
    file_entity: Entity,
    offset: usize,
    root: Entity,
    compiler: &kestrel_compiler::Compiler,
) -> Option<Entity> {
    // Type-position cursor (e.g., `func bar(x: Foo)`).
    let file_cst = compiler.parse(file_entity).tree;
    if let Some((entity, _)) =
        crate::types::type_at_cursor(world, root, &file_cst, file_entity, offset)
    {
        if is_type_entity(world, entity) {
            return Some(entity);
        }
    }

    // Expression-position: resolve Def to a type entity.
    if let Some(body_entity) = semantic::body_entity_at(world, file_entity, offset) {
        let ctx = world.query_context();
        if let Some(hir) = ctx.query(LowerBody {
            entity: body_entity,
            root,
        }) {
            if let Some(expr_id) = semantic::hir_expr_at(&hir, offset) {
                match &hir.exprs[expr_id] {
                    HirExpr::Def(entity, _, _) if is_type_entity(world, *entity) => {
                        return Some(*entity);
                    },
                    HirExpr::MethodCall { .. } | HirExpr::Field { .. } | HirExpr::Call { .. } => {
                        let typed = ctx.query(InferBody {
                            entity: body_entity,
                            root,
                        });
                        if let Some(typed) = typed {
                            if let Some(&resolved) = typed.resolutions.get(&expr_id) {
                                if is_type_entity(world, resolved) {
                                    return Some(resolved);
                                }
                            }
                        }
                    },
                    _ => {},
                }
            }
        }
    }

    // Fallback: cursor on a declaration name.
    let decl = semantic::enclosing_decl_at(world, file_entity, offset)?;
    if is_type_entity(world, decl) {
        Some(decl)
    } else {
        None
    }
}

fn is_type_entity(world: &World, entity: Entity) -> bool {
    matches!(
        world.get::<NodeKind>(entity),
        Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol | NodeKind::TypeAlias)
    )
}

fn entity_to_item(
    world: &World,
    sources: &HashMap<String, String>,
    entity: Entity,
) -> Option<TypeHierarchyItem> {
    let name = world.get::<Name>(entity)?.0.clone();
    let kind = world
        .get::<NodeKind>(entity)
        .and_then(|k| node_kind_to_symbol_kind(k))
        .unwrap_or(SymbolKind::STRUCT);
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

    Some(TypeHierarchyItem {
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

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_compiler::Compiler;

    #[test]
    fn prepare_finds_struct() {
        let src = "module Test\nstruct Point { var x: lang.i64 }\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/th.ks", src.into());
        c.build(f);
        let offset = src.find("Point").unwrap();
        let entity = resolve_type_at(c.world(), f, offset, c.root(), &c);
        assert!(entity.is_some(), "expected type at cursor");
    }

    #[test]
    fn prepare_returns_none_for_function() {
        let src = "module Test\nfunc foo() -> lang.i64 { 1 }\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/th2.ks", src.into());
        c.build(f);
        let offset = src.find("foo").unwrap();
        let entity = resolve_type_at(c.world(), f, offset, c.root(), &c);
        assert!(entity.is_none(), "function should not be a type");
    }
}
