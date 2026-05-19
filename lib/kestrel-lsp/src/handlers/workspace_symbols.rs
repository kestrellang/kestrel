//! `workspace/symbol` — workspace-wide symbol search.
//!
//! Walks all entities with `Name`, `NodeKind`, and `DeclSpan` across all files.
//! Filters by a case-insensitive substring match against the query string.
//! Returns flat `SymbolInformation` results (not nested like document symbols).

use std::collections::HashMap;

use kestrel_ast_builder::{DeclSpan, FilePath, Name, NodeKind};
use kestrel_hecs::{Entity, World};
use tower_lsp::lsp_types::{Location, SymbolInformation, WorkspaceSymbolParams};

use crate::handlers::document_symbols::node_kind_to_symbol_kind;
use crate::position::LineIndex;
use crate::references::entity_file;
use crate::server::{SharedState, path_to_url};

const MAX_RESULTS: usize = 256;

pub async fn handle(
    state: SharedState,
    params: WorkspaceSymbolParams,
) -> Option<Vec<SymbolInformation>> {
    let query = params.query.to_lowercase();

    let (handle, stdlib, user, sources) = {
        let s = state.lock().await;
        let (stdlib, user) = s.partition_sources();
        (s.compiler_handle.clone(), stdlib, user, s.sources.clone())
    };

    handle
        .with_compiler(
            stdlib,
            user,
            move |compiler, _by_path| -> Option<Vec<SymbolInformation>> {
                let world = compiler.world();
                let mut line_indices: HashMap<Entity, LineIndex> = HashMap::new();
                let mut results: Vec<SymbolInformation> = Vec::new();

                for (entity, name) in world.iter_component::<Name>() {
                    if results.len() >= MAX_RESULTS {
                        break;
                    }
                    let name_str = &name.0;
                    if name_str.is_empty() {
                        continue;
                    }
                    if !query.is_empty() && !name_str.to_lowercase().contains(&query) {
                        continue;
                    }

                    let Some(kind) = world.get::<NodeKind>(entity) else {
                        continue;
                    };
                    let Some(symbol_kind) = node_kind_to_symbol_kind(kind) else {
                        continue;
                    };
                    let Some(decl_span) = world.get::<DeclSpan>(entity) else {
                        continue;
                    };

                    let Some(file) = entity_file(world, entity) else {
                        continue;
                    };
                    let Some(file_path) = world.get::<FilePath>(file) else {
                        continue;
                    };
                    let Some(url) = path_to_url(&file_path.0) else {
                        continue;
                    };
                    let Some(source) = sources.get(&file_path.0) else {
                        continue;
                    };

                    let li = line_indices
                        .entry(file)
                        .or_insert_with(|| LineIndex::new(source.clone()));
                    let span = &decl_span.0;
                    let range = li.range_for(span.start, span.end);

                    let container_name = container_name(world, entity);

                    #[allow(deprecated)]
                    results.push(SymbolInformation {
                        name: name_str.clone(),
                        kind: symbol_kind,
                        tags: None,
                        deprecated: None,
                        location: Location { uri: url, range },
                        container_name,
                    });
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

/// Walk the parent chain to find the nearest named ancestor for context.
fn container_name(world: &World, entity: Entity) -> Option<String> {
    let mut cur = world.parent_of(entity);
    while let Some(e) = cur {
        if let Some(name) = world.get::<Name>(e) {
            if !name.0.is_empty() {
                return Some(name.0.clone());
            }
        }
        cur = world.parent_of(e);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_compiler::Compiler;

    fn symbols_for(src: &str, query: &str) -> Vec<SymbolInformation> {
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/ws_sym.ks", src.into());
        c.build(f);
        let world = c.world();
        let mut sources = HashMap::new();
        sources.insert("/tmp/ws_sym.ks".to_string(), src.to_string());
        let query_lower = query.to_lowercase();
        let mut line_indices: HashMap<Entity, LineIndex> = HashMap::new();
        let mut results: Vec<SymbolInformation> = Vec::new();

        for (entity, name) in world.iter_component::<Name>() {
            let name_str = &name.0;
            if name_str.is_empty() {
                continue;
            }
            if !query_lower.is_empty() && !name_str.to_lowercase().contains(&query_lower) {
                continue;
            }
            let Some(kind) = world.get::<NodeKind>(entity) else {
                continue;
            };
            let Some(symbol_kind) = node_kind_to_symbol_kind(kind) else {
                continue;
            };
            let Some(decl_span) = world.get::<DeclSpan>(entity) else {
                continue;
            };
            let Some(file) = entity_file(world, entity) else {
                continue;
            };
            let Some(file_path) = world.get::<FilePath>(file) else {
                continue;
            };
            let Some(url) = path_to_url(&file_path.0) else {
                continue;
            };
            let Some(source) = sources.get(&file_path.0) else {
                continue;
            };
            let li = line_indices
                .entry(file)
                .or_insert_with(|| LineIndex::new(source.clone()));
            let span = &decl_span.0;
            let range = li.range_for(span.start, span.end);
            let container = container_name(world, entity);

            #[allow(deprecated)]
            results.push(SymbolInformation {
                name: name_str.clone(),
                kind: symbol_kind,
                tags: None,
                deprecated: None,
                location: Location { uri: url, range },
                container_name: container,
            });
        }
        results
    }

    #[test]
    fn finds_struct_and_function() {
        let src = "module Test\nstruct Point { var x: lang.i64 }\nfunc origin() -> Point { Point(x: 0) }\n";
        let syms = symbols_for(src, "");
        let names: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Point"), "expected Point in {names:?}");
        assert!(names.contains(&"origin"), "expected origin in {names:?}");
    }

    #[test]
    fn filters_by_query() {
        let src = "module Test\nstruct Alpha {}\nstruct Beta {}\nfunc gamma() -> lang.i64 { 1 }\n";
        let syms = symbols_for(src, "alpha");
        let names: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(
            names.contains(&"Alpha"),
            "case-insensitive match: {names:?}"
        );
        assert!(
            !names.contains(&"Beta"),
            "Beta should be filtered: {names:?}"
        );
        assert!(
            !names.contains(&"gamma"),
            "gamma should be filtered: {names:?}"
        );
    }

    #[test]
    fn container_name_is_parent() {
        let src = "module Test\nstruct Point { var x: lang.i64 }\n";
        let syms = symbols_for(src, "x");
        let x_sym = syms.iter().find(|s| s.name == "x").expect("field x");
        assert_eq!(
            x_sym.container_name.as_deref(),
            Some("Point"),
            "container_name should be Point"
        );
    }
}
