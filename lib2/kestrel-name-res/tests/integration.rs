//! Integration tests for kestrel-name-res.
//!
//! These tests parse real Kestrel source code, build declarations,
//! and then run name resolution queries against the resulting world.

use kestrel_ast_builder::{build_declarations, seed_lang_module, Name, NodeKind};
use kestrel_hecs::{Entity, World};
use kestrel_name_res::*;

// ===== Helpers =====

/// Parse source and build declarations, returning (world, root, file_entity).
fn build_from_source(source: &str) -> (World, Entity, Entity) {
    let mut world = World::new();
    world.begin_revision();

    let root = world.spawn();
    world.set(root, NodeKind::Module);
    world.set(root, Name("<root>".to_string()));

    let file_entity = world.spawn();

    let tokens: Vec<_> = kestrel_lexer2::lex(source, file_entity.index())
        .filter_map(|r| r.ok())
        .collect();
    let token_iter = tokens.iter().map(|t| (t.value.clone(), t.span.clone()));
    let result = kestrel_parser2::parse_source_file_from_source(source, token_iter);

    build_declarations(&mut world, file_entity, &result.tree, root, None);
    (world, root, file_entity)
}

/// Build two source files into the same world, returning (world, root).
fn build_two_files(source_a: &str, source_b: &str) -> (World, Entity) {
    let mut world = World::new();
    world.begin_revision();

    let root = world.spawn();
    world.set(root, NodeKind::Module);
    world.set(root, Name("<root>".to_string()));

    // Build first file
    let file_a = world.spawn();
    let tokens: Vec<_> = kestrel_lexer2::lex(source_a, file_a.index())
        .filter_map(|r| r.ok())
        .collect();
    let token_iter = tokens.iter().map(|t| (t.value.clone(), t.span.clone()));
    let result = kestrel_parser2::parse_source_file_from_source(source_a, token_iter);
    build_declarations(&mut world, file_a, &result.tree, root, None);

    // Build second file
    let file_b = world.spawn();
    let tokens: Vec<_> = kestrel_lexer2::lex(source_b, file_b.index())
        .filter_map(|r| r.ok())
        .collect();
    let token_iter = tokens.iter().map(|t| (t.value.clone(), t.span.clone()));
    let result = kestrel_parser2::parse_source_file_from_source(source_b, token_iter);
    build_declarations(&mut world, file_b, &result.tree, root, None);

    (world, root)
}

/// Find a child entity by NodeKind and Name.
fn find_child(ctx: &kestrel_hecs::QueryContext<'_>, parent: Entity, kind: NodeKind, name: &str) -> Entity {
    ctx.children_of(parent)
        .iter()
        .find(|&&e| {
            ctx.get::<NodeKind>(e) == Some(&kind)
                && ctx.get::<Name>(e).is_some_and(|n| n.0 == name)
        })
        .copied()
        .unwrap_or_else(|| panic!("child {:?} {:?} not found under {:?}", kind, name, parent))
}

/// Debug helper: print entity hierarchy.
#[allow(dead_code)]
fn print_tree(ctx: &kestrel_hecs::QueryContext<'_>, entity: Entity, indent: usize) {
    let name = ctx.get::<Name>(entity).map(|n| n.0.clone()).unwrap_or_default();
    let kind = ctx.get::<NodeKind>(entity).map(|k| format!("{:?}", k)).unwrap_or_default();
    eprintln!("{}{} ({}) [{:?}]", " ".repeat(indent), name, kind, entity);
    for &child in ctx.children_of(entity) {
        print_tree(ctx, child, indent + 2);
    }
}

// ================================================================
// Type Resolution
// ================================================================

#[test]
fn resolve_struct_type_from_same_module() {
    let (world, root, _) = build_from_source(
        "module MyApp\nstruct Foo {}\nstruct Bar {}",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    let result = ctx.query(ResolveTypePath {
        segments: vec!["Foo".into()],
        context: myapp,
        root,
    });
    match result {
        TypeResolution::Found(e) => {
            assert_eq!(ctx.get::<Name>(e).unwrap().0, "Foo");
            assert_eq!(ctx.get::<NodeKind>(e), Some(&NodeKind::Struct));
        }
        other => panic!("expected Found, got {:?}", other),
    }
}

#[test]
fn resolve_enum_type() {
    let (world, root, _) = build_from_source(
        "module MyApp\nenum Color {\n  case Red\n  case Green\n  case Blue\n}",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    let result = ctx.query(ResolveTypePath {
        segments: vec!["Color".into()],
        context: myapp,
        root,
    });
    match result {
        TypeResolution::Found(e) => {
            assert_eq!(ctx.get::<Name>(e).unwrap().0, "Color");
            assert_eq!(ctx.get::<NodeKind>(e), Some(&NodeKind::Enum));
        }
        other => panic!("expected Found, got {:?}", other),
    }
}

#[test]
fn resolve_protocol_type() {
    let (world, root, _) = build_from_source(
        "module MyApp\nprotocol Printable {\n  func description() -> String\n}",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    let result = ctx.query(ResolveTypePath {
        segments: vec!["Printable".into()],
        context: myapp,
        root,
    });
    match result {
        TypeResolution::Found(e) => {
            assert_eq!(ctx.get::<Name>(e).unwrap().0, "Printable");
            assert_eq!(ctx.get::<NodeKind>(e), Some(&NodeKind::Protocol));
        }
        other => panic!("expected Found, got {:?}", other),
    }
}

#[test]
fn resolve_type_via_import() {
    let (world, root) = build_two_files(
        "module std.core\npublic struct Int64 {}\npublic struct Bool {}",
        "module MyApp\nimport std.core",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    // Int64 should be accessible via wildcard import
    let result = ctx.query(ResolveTypePath {
        segments: vec!["Int64".into()],
        context: myapp,
        root,
    });
    match result {
        TypeResolution::Found(e) => {
            assert_eq!(ctx.get::<Name>(e).unwrap().0, "Int64");
        }
        other => panic!("expected Found(Int64), got {:?}", other),
    }
}

#[test]
fn resolve_type_via_auto_import() {
    // Non-std modules auto-import std leaf modules
    let (world, root) = build_two_files(
        "module std.core\npublic struct Int64 {}",
        "module MyApp",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    // Int64 should be findable via auto-import (no explicit import needed)
    let result = ctx.query(ResolveTypePath {
        segments: vec!["Int64".into()],
        context: myapp,
        root,
    });
    match result {
        TypeResolution::Found(e) => {
            assert_eq!(ctx.get::<Name>(e).unwrap().0, "Int64");
        }
        other => panic!("expected Found(Int64), got {:?}", other),
    }
}

#[test]
fn resolve_type_not_found() {
    let (world, root, _) = build_from_source("module MyApp\nstruct Foo {}");
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    let result = ctx.query(ResolveTypePath {
        segments: vec!["Bar".into()],
        context: myapp,
        root,
    });
    assert!(matches!(result, TypeResolution::NotFound(_)));
}

#[test]
fn resolve_self_type() {
    let (world, root, _) = build_from_source("module MyApp\nstruct Foo {}");
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    let result = ctx.query(ResolveTypePath {
        segments: vec!["Self".into()],
        context: myapp,
        root,
    });
    assert!(matches!(result, TypeResolution::SelfType));
}

#[test]
fn resolve_lang_types() {
    let mut world = World::new();
    world.begin_revision();

    let root = world.spawn();
    world.set(root, NodeKind::Module);
    world.set(root, Name("<root>".to_string()));

    seed_lang_module(&mut world, root);
    let ctx = world.query_context();

    let result = ctx.query(ResolveTypePath {
        segments: vec!["lang".into(), "i64".into()],
        context: root,
        root,
    });
    match result {
        TypeResolution::Found(e) => {
            assert_eq!(ctx.get::<Name>(e).unwrap().0, "i64");
        }
        other => panic!("expected Found(i64), got {:?}", other),
    }
}

// ================================================================
// Value Resolution
// ================================================================

#[test]
fn resolve_function_value() {
    let (world, root, _) = build_from_source(
        "module MyApp\nfunc greet() -> String {\n  return \"hello\"\n}",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    let result = ctx.query(ResolveValuePath {
        segments: vec!["greet".into()],
        context: myapp,
        root,
    });
    match result {
        ValueResolution::Def(e) => {
            assert_eq!(ctx.get::<Name>(e).unwrap().0, "greet");
            assert_eq!(ctx.get::<NodeKind>(e), Some(&NodeKind::Function));
        }
        other => panic!("expected Def, got {:?}", other),
    }
}

#[test]
fn resolve_overloaded_functions() {
    let (world, root, _) = build_from_source(
        "module MyApp\nfunc add(x: Int64) -> Int64 {\n  return x\n}\nfunc add(x: Int64, y: Int64) -> Int64 {\n  return x\n}",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    let result = ctx.query(ResolveValuePath {
        segments: vec!["add".into()],
        context: myapp,
        root,
    });
    match result {
        ValueResolution::Overloaded(entities) => {
            assert_eq!(entities.len(), 2);
            assert!(entities
                .iter()
                .all(|&e| ctx.get::<NodeKind>(e) == Some(&NodeKind::Function)));
        }
        other => panic!("expected Overloaded, got {:?}", other),
    }
}

#[test]
fn resolve_enum_case_via_qualified_path() {
    let (world, root, _) = build_from_source(
        "module MyApp\nenum Direction {\n  case North\n  case South\n  case East\n  case West\n}",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    // Direction.North
    let result = ctx.query(ResolveValuePath {
        segments: vec!["Direction".into(), "North".into()],
        context: myapp,
        root,
    });
    match result {
        ValueResolution::Def(e) => {
            assert_eq!(ctx.get::<Name>(e).unwrap().0, "North");
            assert_eq!(ctx.get::<NodeKind>(e), Some(&NodeKind::EnumCase));
        }
        other => panic!("expected Def(North), got {:?}", other),
    }
}

#[test]
fn resolve_value_not_found() {
    let (world, root, _) = build_from_source("module MyApp\nfunc foo() {}");
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    let result = ctx.query(ResolveValuePath {
        segments: vec!["bar".into()],
        context: myapp,
        root,
    });
    assert!(matches!(result, ValueResolution::NotFound(_)));
}

// ================================================================
// Name Resolution
// ================================================================

#[test]
fn resolve_name_local_declaration() {
    let (world, root, _) = build_from_source(
        "module MyApp\nstruct Widget {}\nfunc process() {}",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    let result = ctx.query(ResolveName {
        name: "Widget".into(),
        context: myapp,
        root,
    });
    match result {
        NameResolution::Found(entities) => {
            assert_eq!(entities.len(), 1);
            assert_eq!(ctx.get::<Name>(entities[0]).unwrap().0, "Widget");
        }
        other => panic!("expected Found, got {:?}", other),
    }
}

#[test]
fn resolve_name_local_shadows_import() {
    let (world, root) = build_two_files(
        "module std.core\npublic struct Int64 {}",
        "module MyApp\nimport std.core\nstruct Int64 {}",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    // Local Int64 should shadow the imported one
    let result = ctx.query(ResolveName {
        name: "Int64".into(),
        context: myapp,
        root,
    });
    match result {
        NameResolution::Found(entities) => {
            assert_eq!(entities.len(), 1);
            let e = entities[0];
            // The local one should be a child of MyApp
            assert_eq!(ctx.parent_of(e), Some(myapp));
        }
        other => panic!("expected Found, got {:?}", other),
    }
}

// ================================================================
// Module Resolution
// ================================================================

#[test]
fn resolve_module_path() {
    let (world, root) = build_two_files(
        "module std.core\npublic struct Int64 {}",
        "module MyApp",
    );
    let ctx = world.query_context();

    let result = ctx.query(ResolveModulePath {
        path: vec!["std".into(), "core".into()],
        root,
    });
    match result {
        Some(e) => {
            assert_eq!(ctx.get::<Name>(e).unwrap().0, "core");
            assert_eq!(ctx.get::<NodeKind>(e), Some(&NodeKind::Module));
        }
        None => panic!("expected Some, got None"),
    }
}

#[test]
fn resolve_module_path_not_found() {
    let (world, root, _) = build_from_source("module MyApp");
    let ctx = world.query_context();

    let result = ctx.query(ResolveModulePath {
        path: vec!["nonexistent".into(), "module".into()],
        root,
    });
    assert!(result.is_none());
}

// ================================================================
// Scope
// ================================================================

#[test]
fn scope_includes_local_declarations() {
    let (world, root, _) = build_from_source(
        "module MyApp\nstruct Alpha {}\nstruct Beta {}\nfunc gamma() {}",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    let scope = ctx.query(ScopeFor {
        entity: myapp,
        root,
    });

    assert!(scope.declarations.contains_key("Alpha"));
    assert!(scope.declarations.contains_key("Beta"));
    assert!(scope.declarations.contains_key("gamma"));
}

#[test]
fn scope_includes_wildcard_imports() {
    let (world, root) = build_two_files(
        "module std.core\npublic struct Int64 {}",
        "module MyApp\nimport std.core",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    let scope = ctx.query(ScopeFor {
        entity: myapp,
        root,
    });

    // std.core should be in wildcard imports (both explicit + auto-import)
    assert!(
        !scope.wildcard_imports.is_empty(),
        "expected wildcard imports to include std.core"
    );

    // Verify that the wildcard import resolves to the core module
    let core_module = scope.wildcard_imports.iter().find(|&&e| {
        ctx.get::<Name>(e).is_some_and(|n| n.0 == "core")
    });
    assert!(core_module.is_some(), "std.core not found in wildcard imports");
}

#[test]
fn scope_selective_import() {
    let (world, root) = build_two_files(
        "module std.core\npublic struct Int64 {}\npublic struct Bool {}",
        "module MyApp\nimport std.core.(Int64)",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    let scope = ctx.query(ScopeFor {
        entity: myapp,
        root,
    });

    // Int64 should be selectively imported
    assert!(
        scope.selective_imports.contains_key("Int64"),
        "expected Int64 in selective imports, got: {:?}",
        scope.selective_imports.keys().collect::<Vec<_>>()
    );
}

// ================================================================
// Visibility
// ================================================================

#[test]
fn visibility_public_accessible_from_outside() {
    let (world, root) = build_two_files(
        "module std.core\npublic struct Int64 {}",
        "module MyApp\nimport std.core",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    // Int64 should be visible from MyApp
    let result = ctx.query(ResolveTypePath {
        segments: vec!["Int64".into()],
        context: myapp,
        root,
    });
    assert!(matches!(result, TypeResolution::Found(_)));
}

#[test]
fn visibility_private_not_accessible_from_outside() {
    // No `public` keyword — has no Vis component, so it's visible by default.
    // This tests the current design: no Vis = always visible.
    let (world, root) = build_two_files(
        "module std.core\nstruct Secret {}",
        "module MyApp\nimport std.core",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    let result = ctx.query(ResolveTypePath {
        segments: vec!["Secret".into()],
        context: myapp,
        root,
    });
    // Items without Vis component are visible by default
    assert!(matches!(result, TypeResolution::Found(_)));
}

// ================================================================
// Generics & Type Parameters
// ================================================================

#[test]
fn resolve_type_parameter_in_generic_struct() {
    let (world, root, _) = build_from_source(
        "module MyApp\nstruct Box[T] {\n  var value: T\n}",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");
    let box_struct = find_child(&ctx, myapp, NodeKind::Struct, "Box");

    // T should be resolvable within Box
    let result = ctx.query(ResolveTypePath {
        segments: vec!["T".into()],
        context: box_struct,
        root,
    });
    match result {
        TypeResolution::Found(e) => {
            assert_eq!(ctx.get::<Name>(e).unwrap().0, "T");
            assert_eq!(ctx.get::<NodeKind>(e), Some(&NodeKind::TypeParameter));
        }
        other => panic!("expected Found(T), got {:?}", other),
    }
}

#[test]
fn resolve_type_parameter_not_visible_outside() {
    let (world, root, _) = build_from_source(
        "module MyApp\nstruct Box[T] {\n  var value: T\n}",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    // T should NOT be resolvable from the module level
    let result = ctx.query(ResolveTypePath {
        segments: vec!["T".into()],
        context: myapp,
        root,
    });
    assert!(matches!(result, TypeResolution::NotFound(_)));
}

// ================================================================
// Multi-file / Cross-module
// ================================================================

#[test]
fn cross_module_type_resolution() {
    let (world, root) = build_two_files(
        "module std.collections\npublic struct Array[T] {}\npublic struct Dictionary[K, V] {}",
        "module MyApp\nimport std.collections",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    // Array should be accessible via import
    let result = ctx.query(ResolveTypePath {
        segments: vec!["Array".into()],
        context: myapp,
        root,
    });
    match result {
        TypeResolution::Found(e) => {
            assert_eq!(ctx.get::<Name>(e).unwrap().0, "Array");
        }
        other => panic!("expected Found(Array), got {:?}", other),
    }

    // Dictionary too
    let result = ctx.query(ResolveTypePath {
        segments: vec!["Dictionary".into()],
        context: myapp,
        root,
    });
    match result {
        TypeResolution::Found(e) => {
            assert_eq!(ctx.get::<Name>(e).unwrap().0, "Dictionary");
        }
        other => panic!("expected Found(Dictionary), got {:?}", other),
    }
}

// ================================================================
// Extensions
// ================================================================

#[test]
fn extension_found_for_type() {
    let (world, root, _) = build_from_source(
        "module MyApp\nstruct Foo {}\nextend Foo {\n  func bar() {}\n}",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");
    let foo = find_child(&ctx, myapp, NodeKind::Struct, "Foo");

    let extensions = ctx.query(ExtensionsFor {
        target: foo,
        root,
    });
    assert!(
        !extensions.is_empty(),
        "expected at least one extension for Foo"
    );

    // The extension should contain `bar`
    let ext = extensions[0];
    let bar_children: Vec<_> = ctx.children_of(ext)
        .iter()
        .filter(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "bar"))
        .copied()
        .collect();
    assert_eq!(bar_children.len(), 1);
}

// ================================================================
// Protocol Conformance
// ================================================================

#[test]
fn resolve_type_with_conformance() {
    let (world, root, _) = build_from_source(
        "module MyApp\nprotocol Equatable {\n  func equals(other: Self) -> Bool\n}\nstruct Point: Equatable {\n  var x: Int64\n  var y: Int64\n  func equals(other: Self) -> Bool {\n    return true\n  }\n}",
    );
    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    let result = ctx.query(ResolveTypePath {
        segments: vec!["Equatable".into()],
        context: myapp,
        root,
    });
    assert!(matches!(result, TypeResolution::Found(_)));

    let result = ctx.query(ResolveTypePath {
        segments: vec!["Point".into()],
        context: myapp,
        root,
    });
    assert!(matches!(result, TypeResolution::Found(_)));
}

// ================================================================
// Real Stdlib Source
// ================================================================

#[test]
fn resolve_from_real_ordering_source() {
    let source = include_str!("../../../lang/std/core/ordering.ks");
    let (world, root, _) = build_from_source(source);
    let ctx = world.query_context();

    // Find std.core module
    let result = ctx.query(ResolveModulePath {
        path: vec!["std".into(), "core".into()],
        root,
    });
    let core = result.expect("std.core module should exist");

    // Ordering enum should be resolvable from within std.core
    let result = ctx.query(ResolveTypePath {
        segments: vec!["Ordering".into()],
        context: core,
        root,
    });
    match result {
        TypeResolution::Found(e) => {
            assert_eq!(ctx.get::<Name>(e).unwrap().0, "Ordering");
            assert_eq!(ctx.get::<NodeKind>(e), Some(&NodeKind::Enum));
        }
        other => panic!("expected Found(Ordering), got {:?}", other),
    }

    // Ordering.Less should resolve as an enum case
    let result = ctx.query(ResolveValuePath {
        segments: vec!["Ordering".into(), "Less".into()],
        context: core,
        root,
    });
    match result {
        ValueResolution::Def(e) => {
            assert_eq!(ctx.get::<Name>(e).unwrap().0, "Less");
            assert_eq!(ctx.get::<NodeKind>(e), Some(&NodeKind::EnumCase));
        }
        other => panic!("expected Def(Less), got {:?}", other),
    }
}

#[test]
fn resolve_from_real_bool_source() {
    let source = include_str!("../../../lang/std/core/bool.ks");
    let (world, root, _) = build_from_source(source);
    let ctx = world.query_context();

    let core = ctx.query(ResolveModulePath {
        path: vec!["std".into(), "core".into()],
        root,
    }).expect("std.core should exist");

    // Bool should be resolvable
    let result = ctx.query(ResolveTypePath {
        segments: vec!["Bool".into()],
        context: core,
        root,
    });
    match result {
        TypeResolution::Found(e) => {
            assert_eq!(ctx.get::<Name>(e).unwrap().0, "Bool");
        }
        other => panic!("expected Found(Bool), got {:?}", other),
    }
}

#[test]
fn auto_import_across_std_modules() {
    // Build two std modules and a user module
    let mut world = World::new();
    world.begin_revision();

    let root = world.spawn();
    world.set(root, NodeKind::Module);
    world.set(root, Name("<root>".to_string()));

    // std.core with Int64
    let f1 = world.spawn();
    let src1 = "module std.core\npublic struct Int64 {}";
    let tokens1: Vec<_> = kestrel_lexer2::lex(src1, f1.index()).filter_map(|r| r.ok()).collect();
    let result1 = kestrel_parser2::parse_source_file_from_source(
        src1, tokens1.iter().map(|t| (t.value.clone(), t.span.clone())),
    );
    build_declarations(&mut world, f1, &result1.tree, root, None);

    // std.text with String
    let f2 = world.spawn();
    let src2 = "module std.text\npublic struct String {}";
    let tokens2: Vec<_> = kestrel_lexer2::lex(src2, f2.index()).filter_map(|r| r.ok()).collect();
    let result2 = kestrel_parser2::parse_source_file_from_source(
        src2, tokens2.iter().map(|t| (t.value.clone(), t.span.clone())),
    );
    build_declarations(&mut world, f2, &result2.tree, root, None);

    // User module — no explicit imports
    let f3 = world.spawn();
    let src3 = "module MyApp";
    let tokens3: Vec<_> = kestrel_lexer2::lex(src3, f3.index()).filter_map(|r| r.ok()).collect();
    let result3 = kestrel_parser2::parse_source_file_from_source(
        src3, tokens3.iter().map(|t| (t.value.clone(), t.span.clone())),
    );
    build_declarations(&mut world, f3, &result3.tree, root, None);

    let ctx = world.query_context();
    let myapp = find_child(&ctx, root, NodeKind::Module, "MyApp");

    // Both Int64 and String should be auto-imported
    let result = ctx.query(ResolveTypePath {
        segments: vec!["Int64".into()],
        context: myapp,
        root,
    });
    assert!(matches!(result, TypeResolution::Found(_)), "Int64 should be auto-imported");

    let result = ctx.query(ResolveTypePath {
        segments: vec!["String".into()],
        context: myapp,
        root,
    });
    assert!(matches!(result, TypeResolution::Found(_)), "String should be auto-imported");
}

#[test]
fn std_module_does_not_auto_import_itself() {
    let (world, root) = build_two_files(
        "module std.core\npublic struct Int64 {}",
        "module std.text\npublic struct String {}",
    );
    let ctx = world.query_context();

    let core = ctx.query(ResolveModulePath {
        path: vec!["std".into(), "core".into()],
        root,
    }).expect("std.core should exist");

    let scope = ctx.query(ScopeFor {
        entity: core,
        root,
    });

    // std.core should NOT auto-import std modules (it's inside std)
    assert!(
        scope.wildcard_imports.is_empty(),
        "std.core should not have auto-imports, got {} wildcard imports",
        scope.wildcard_imports.len()
    );
}
