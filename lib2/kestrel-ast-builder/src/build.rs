//! Entry point for building declaration entities from a CST.
//!
//! Walks the CST using an iterative stack, dispatching to per-declaration
//! builder functions. Container types (struct/enum/protocol/extension) push
//! their body children onto the stack for processing.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode};

use crate::components::{Os, TargetConfig};

use crate::builders::{
    enum_decl, extension, field, function, import, module, protocol, struct_decl, subscript,
    type_alias,
};

/// Build declaration entities from a parsed CST.
///
/// Walks the syntax tree and creates entities with components in the ECS
/// world. Container declarations (struct, enum, protocol, extension) have
/// their members processed as children in the hierarchy.
///
/// # Arguments
/// * `world` - ECS world to populate
/// * `file_entity` - Entity handle for the source file
/// * `tree` - Root SyntaxNode (SourceFile) from parsing
/// * `root` - Root module entity (parent for top-level declarations)
/// * `target` - Compilation target for conditional filtering (`@platform`, etc.).
///   Pass `None` to include all declarations regardless of platform attributes.
pub fn build_declarations(
    world: &mut World,
    file_entity: Entity,
    tree: &SyntaxNode,
    root: Entity,
    target: Option<&TargetConfig>,
) {
    let file_id = file_entity.index();

    // Find the module for this file (from ModuleDeclaration if present)
    let module_parent = tree
        .children()
        .find(|c| c.kind() == SyntaxKind::ModuleDeclaration)
        .map(|mod_node| module::resolve_module_path(world, root, &mod_node))
        .unwrap_or(root);

    // Stack-based iteration: (node, parent_entity)
    let mut stack: Vec<(SyntaxNode, Entity)> = Vec::new();

    // Push top-level children (excluding the module declaration) in reverse
    // order so they're processed left-to-right
    let top_level: Vec<_> = tree
        .children()
        .filter(|c| c.kind() != SyntaxKind::ModuleDeclaration)
        .collect();

    for child in top_level.into_iter().rev() {
        stack.push((child, module_parent));
    }

    // Process the stack
    while let Some((node, parent)) = stack.pop() {
        // Skip declarations excluded by target-conditional attributes (@platform, etc.)
        if is_excluded_by_target(&node, target) {
            continue;
        }

        match node.kind() {
            SyntaxKind::StructDeclaration => {
                let (entity, body) =
                    struct_decl::build_struct(world, &node, parent, file_entity, file_id);
                push_body_children(&mut stack, body, entity);
            }

            SyntaxKind::EnumDeclaration => {
                let (entity, body) =
                    enum_decl::build_enum(world, &node, parent, file_entity, file_id);
                push_body_children(&mut stack, body, entity);
            }

            SyntaxKind::EnumCaseDeclaration => {
                enum_decl::build_enum_case(world, &node, parent, file_entity, file_id);
            }

            SyntaxKind::ProtocolDeclaration => {
                let (entity, body) =
                    protocol::build_protocol(world, &node, parent, file_entity, file_id);
                push_body_children(&mut stack, body, entity);
            }

            SyntaxKind::ExtensionDeclaration => {
                let (entity, body) =
                    extension::build_extension(world, &node, parent, file_entity, file_id);
                push_body_children(&mut stack, body, entity);
            }

            SyntaxKind::FunctionDeclaration => {
                function::build_function(world, &node, parent, file_entity, file_id);
            }

            SyntaxKind::InitializerDeclaration => {
                function::build_initializer(world, &node, parent, file_entity, file_id);
            }

            SyntaxKind::DeinitDeclaration => {
                function::build_deinit(world, &node, parent, file_entity, file_id);
            }

            SyntaxKind::FieldDeclaration => {
                field::build_field(world, &node, parent, file_entity, file_id);
            }

            SyntaxKind::SubscriptDeclaration => {
                subscript::build_subscript(world, &node, parent, file_entity, file_id);
            }

            SyntaxKind::TypeAliasDeclaration => {
                type_alias::build_type_alias(world, &node, parent, file_entity, file_id);
            }

            SyntaxKind::ImportDeclaration => {
                import::build_import(world, &node, parent, file_entity, file_id);
            }

            // Transparent wrapper nodes — push children through
            SyntaxKind::DeclarationItem | SyntaxKind::SourceFile => {
                let children: Vec<_> = node.children().collect();
                for child in children.into_iter().rev() {
                    stack.push((child, parent));
                }
            }

            // Unknown nodes — skip silently
            _ => {}
        }
    }
}

/// Push children of a body node onto the stack in reverse order.
fn push_body_children(
    stack: &mut Vec<(SyntaxNode, Entity)>,
    body: Option<SyntaxNode>,
    parent: Entity,
) {
    if let Some(body) = body {
        let children: Vec<_> = body.children().collect();
        for child in children.into_iter().rev() {
            stack.push((child, parent));
        }
    }
}

/// Check if a CST node should be excluded based on target-conditional attributes.
/// Scans the node's AttributeList for conditional attributes (@platform, etc.)
/// and compares each against the corresponding TargetConfig field.
/// Excluded if ANY conditional attribute doesn't match the target.
fn is_excluded_by_target(node: &SyntaxNode, target: Option<&TargetConfig>) -> bool {
    let Some(target) = target else {
        return false;
    };

    let Some(attr_list) = node
        .children()
        .find(|c| c.kind() == SyntaxKind::AttributeList)
    else {
        return false;
    };

    for attr_node in attr_list
        .children()
        .filter(|c| c.kind() == SyntaxKind::Attribute)
    {
        let attr_name = attr_node
            .children_with_tokens()
            .filter_map(|c| c.into_token())
            .find(|tok| tok.kind() == SyntaxKind::Identifier)
            .map(|tok| tok.text().to_string());

        let Some(name) = attr_name else { continue };

        // Check each conditional attribute type against its target dimension
        match name.as_str() {
            "platform" => {
                if is_excluded_by_platform(&attr_node, target) {
                    return true;
                }
            }
            // Future: "arch" => { if is_excluded_by_arch(...) { return true; } }
            _ => {}
        }
    }

    false
}

/// Check if a @platform attribute excludes this declaration for the given target.
fn is_excluded_by_platform(attr_node: &SyntaxNode, target: &TargetConfig) -> bool {
    let Some(target_os) = target.os else {
        return false; // no OS target set — don't filter
    };

    let Some(args_node) = attr_node
        .children()
        .find(|c| c.kind() == SyntaxKind::AttributeArgs)
    else {
        return false; // @platform with no args — let validation report
    };

    // Extract the implicit member value from the first arg
    for arg_node in args_node
        .children()
        .filter(|c| c.kind() == SyntaxKind::AttributeArg)
    {
        let tokens: Vec<_> = arg_node
            .children_with_tokens()
            .filter_map(|c| c.into_token())
            .collect();

        let mut found_dot = false;
        for tok in &tokens {
            if tok.kind() == SyntaxKind::Dot {
                found_dot = true;
            } else if found_dot && tok.kind() == SyntaxKind::Identifier {
                return match Os::from_name(tok.text()) {
                    Some(declared) => declared != target_os,
                    None => false, // unknown platform — let validation report
                };
            }
        }

        return false; // no implicit member — don't exclude
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_hecs::World;
    use crate::ast_type::AstType;
    use crate::components::*;

    /// Helper: parse source and build declarations, returning world + root + file entity
    fn build_from_source(source: &str) -> (World, Entity, Entity) {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".to_string()));

        let file_entity = world.spawn();

        // Lex and parse
        let tokens: Vec<_> = kestrel_lexer2::lex(source, file_entity.index())
            .filter_map(|r| r.ok())
            .collect();
        let token_iter = tokens.iter().map(|t| (t.value.clone(), t.span.clone()));
        let result = kestrel_parser2::parse_source_file_from_source(source, token_iter);

        build_declarations(&mut world, file_entity, &result.tree, root, None);
        (world, root, file_entity)
    }

    /// Find a child entity with matching NodeKind and Name.
    fn find_child_by_name(world: &World, parent: Entity, kind: &NodeKind, name: &str) -> Option<Entity> {
        world.children_of(parent).iter().find(|&&e| {
            world.get::<NodeKind>(e) == Some(kind)
                && world.get::<Name>(e).is_some_and(|n| n.0 == name)
        }).copied()
    }

    /// Find first child entity with matching NodeKind.
    fn find_child_by_kind(world: &World, parent: Entity, kind: &NodeKind) -> Option<Entity> {
        world.children_of(parent).iter().find(|&&e| {
            world.get::<NodeKind>(e) == Some(kind)
        }).copied()
    }

    // ================================================================
    // 1. Module hierarchy
    // ================================================================

    #[test]
    fn module_hierarchy_nested() {
        let (world, root, _) = build_from_source("module Foo.Bar\nstruct A {}");

        let foo = find_child_by_name(&world, root, &NodeKind::Module, "Foo").unwrap();
        let bar = find_child_by_name(&world, foo, &NodeKind::Module, "Bar").unwrap();
        let a = find_child_by_name(&world, bar, &NodeKind::Struct, "A").unwrap();

        assert!(world.get::<FileId>(foo).is_none(), "modules have no FileId");
        assert!(world.get::<FileId>(a).is_some(), "struct has FileId");
    }

    // ================================================================
    // 2. Cross-file module reuse
    // ================================================================

    #[test]
    fn cross_file_module_reuse() {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".to_string()));

        // File 1
        let f1 = world.spawn();
        let src1 = "module Shared\nstruct A {}";
        let tokens1: Vec<_> = kestrel_lexer2::lex(src1, f1.index()).filter_map(|r| r.ok()).collect();
        let result1 = kestrel_parser2::parse_source_file_from_source(
            src1,
            tokens1.iter().map(|t| (t.value.clone(), t.span.clone())),
        );
        build_declarations(&mut world, f1, &result1.tree, root, None);

        // File 2
        let f2 = world.spawn();
        let src2 = "module Shared\nstruct B {}";
        let tokens2: Vec<_> = kestrel_lexer2::lex(src2, f2.index()).filter_map(|r| r.ok()).collect();
        let result2 = kestrel_parser2::parse_source_file_from_source(
            src2,
            tokens2.iter().map(|t| (t.value.clone(), t.span.clone())),
        );
        build_declarations(&mut world, f2, &result2.tree, root, None);

        // Both files should share the same module entity
        let shared = find_child_by_name(&world, root, &NodeKind::Module, "Shared").unwrap();
        assert_eq!(world.children_of(shared).len(), 2, "both structs under same module");

        let a = find_child_by_name(&world, shared, &NodeKind::Struct, "A");
        let b = find_child_by_name(&world, shared, &NodeKind::Struct, "B");
        assert!(a.is_some());
        assert!(b.is_some());
    }

    // ================================================================
    // 3. Struct with fields
    // ================================================================

    #[test]
    fn struct_with_fields() {
        let (world, root, file) = build_from_source(
            "module Main\npublic struct Point {\n  var x: Int64\n  let y: String\n}",
        );

        let main = find_child_by_name(&world, root, &NodeKind::Module, "Main").unwrap();
        let point = find_child_by_name(&world, main, &NodeKind::Struct, "Point").unwrap();

        assert!(world.has::<Typed>(point));
        assert_eq!(world.get::<Vis>(point), Some(&Vis::Public));
        assert_eq!(world.get::<FileId>(point), Some(&FileId(file)));

        let x = find_child_by_name(&world, point, &NodeKind::Field, "x").unwrap();
        assert!(world.has::<Gettable>(x));
        assert!(world.has::<Settable>(x), "var field is Settable");
        assert!(world.get::<TypeAnnotation>(x).is_some());

        let y = find_child_by_name(&world, point, &NodeKind::Field, "y").unwrap();
        assert!(world.has::<Gettable>(y));
        assert!(!world.has::<Settable>(y), "let field is not Settable");
    }

    // ================================================================
    // 4. Enum with cases
    // ================================================================

    #[test]
    fn enum_with_cases() {
        let (world, root, _) = build_from_source(
            "module Main\nindirect enum Shape {\n  case circle(radius: Float64)\n  case rectangle(width: Float64, height: Float64)\n  case point\n}",
        );

        let main = find_child_by_name(&world, root, &NodeKind::Module, "Main").unwrap();
        let shape = find_child_by_name(&world, main, &NodeKind::Enum, "Shape").unwrap();

        assert!(world.has::<Typed>(shape));
        assert!(world.has::<IsIndirect>(shape));

        let circle = find_child_by_name(&world, shape, &NodeKind::EnumCase, "circle").unwrap();
        let callable = world.get::<Callable>(circle).unwrap();
        assert_eq!(callable.params.len(), 1);
        assert_eq!(callable.params[0].label.as_deref(), Some("radius"));

        let rect = find_child_by_name(&world, shape, &NodeKind::EnumCase, "rectangle").unwrap();
        let callable = world.get::<Callable>(rect).unwrap();
        assert_eq!(callable.params.len(), 2);

        let point = find_child_by_name(&world, shape, &NodeKind::EnumCase, "point").unwrap();
        assert!(!world.has::<Callable>(point), "case without values has no Callable");
    }

    // ================================================================
    // 5. Function
    // ================================================================

    #[test]
    fn function_with_body_and_return() {
        let (world, root, _) = build_from_source(
            "module Main\nstruct Foo {\n  static mutating func bar(x: Int64) -> String {}\n}",
        );

        let main = find_child_by_name(&world, root, &NodeKind::Module, "Main").unwrap();
        let foo = find_child_by_name(&world, main, &NodeKind::Struct, "Foo").unwrap();
        let bar = find_child_by_name(&world, foo, &NodeKind::Function, "bar").unwrap();

        // Callable with receiver
        let callable = world.get::<Callable>(bar).unwrap();
        assert_eq!(callable.params.len(), 1);
        assert_eq!(callable.params[0].name, "x");
        // Static functions have no receiver, even with mutating keyword
        assert_eq!(callable.receiver, None);

        // Return type
        assert!(world.get::<TypeAnnotation>(bar).is_some());

        // Body
        assert!(world.has::<Valued>(bar));

        // Static
        assert!(world.has::<Static>(bar));
    }

    // ================================================================
    // 6. Computed property
    // ================================================================

    #[test]
    fn computed_property_gettable_settable() {
        let (world, root, _) = build_from_source(
            "module Main\nstruct S {\n  var computed: Int64 {\n    get {}\n    set {}\n  }\n}",
        );

        let main = find_child_by_name(&world, root, &NodeKind::Module, "Main").unwrap();
        let s = find_child_by_name(&world, main, &NodeKind::Struct, "S").unwrap();
        let prop = find_child_by_name(&world, s, &NodeKind::Field, "computed").unwrap();

        assert!(world.has::<Gettable>(prop));
        assert!(world.has::<Settable>(prop));
    }

    // ================================================================
    // 7. Type parameters
    // ================================================================

    #[test]
    fn type_parameters() {
        let (world, root, _) = build_from_source(
            "module Main\nstruct Box[T] {}",
        );

        let main = find_child_by_name(&world, root, &NodeKind::Module, "Main").unwrap();
        let box_entity = find_child_by_name(&world, main, &NodeKind::Struct, "Box").unwrap();

        let type_params = world.get::<TypeParams>(box_entity).unwrap();
        assert_eq!(type_params.0.len(), 1);

        let tp = type_params.0[0];
        assert_eq!(world.get::<NodeKind>(tp), Some(&NodeKind::TypeParameter));
        assert_eq!(world.get::<Name>(tp).unwrap().0, "T");
        assert_eq!(world.parent_of(tp), Some(box_entity));
    }

    // ================================================================
    // 8. Where clause
    // ================================================================

    #[test]
    fn where_clause() {
        let (world, root, _) = build_from_source(
            "module Main\nfunc process[T](x: T) where T: Comparable {}",
        );

        let main = find_child_by_name(&world, root, &NodeKind::Module, "Main").unwrap();
        let func = find_child_by_name(&world, main, &NodeKind::Function, "process").unwrap();

        let wc = world.get::<WhereClause>(func);
        assert!(wc.is_some(), "function should have WhereClause");
        let constraints = &wc.unwrap().0;
        assert!(!constraints.is_empty());
    }

    // ================================================================
    // 9. Conformances
    // ================================================================

    #[test]
    fn conformances() {
        let (world, root, _) = build_from_source(
            "module Main\nstruct S: Hashable, Comparable {}",
        );

        let main = find_child_by_name(&world, root, &NodeKind::Module, "Main").unwrap();
        let s = find_child_by_name(&world, main, &NodeKind::Struct, "S").unwrap();

        let conf = world.get::<Conformances>(s).unwrap();
        assert_eq!(conf.0.len(), 2);

        // Both should be positive
        assert!(conf.0.iter().all(|c| matches!(c, ConformanceItem::Positive(..))));
    }

    // ================================================================
    // 10. Extension
    // ================================================================

    #[test]
    fn extension_has_target_no_name() {
        let (world, root, _) = build_from_source(
            "module Main\nextend Int64: Hashable {}",
        );

        let main = find_child_by_name(&world, root, &NodeKind::Module, "Main").unwrap();
        let ext = find_child_by_kind(&world, main, &NodeKind::Extension).unwrap();

        assert!(!world.has::<Name>(ext), "extensions have no Name");
        assert!(world.get::<ExtensionTarget>(ext).is_some());
        assert!(world.get::<Conformances>(ext).is_some());
    }

    // ================================================================
    // 11. Import
    // ================================================================

    #[test]
    fn import_with_items() {
        let (world, root, _) = build_from_source(
            "module Main\nimport std.collections.(Array, Dictionary)",
        );

        let main = find_child_by_name(&world, root, &NodeKind::Module, "Main").unwrap();
        let imp = find_child_by_kind(&world, main, &NodeKind::Import).unwrap();

        let path = world.get::<ModulePath>(imp).unwrap();
        assert_eq!(path.0, vec!["std", "collections"]);

        let items = world.get::<ImportItems>(imp).unwrap();
        assert_eq!(items.0.len(), 2);
        assert_eq!(items.0[0].name, "Array");
        assert_eq!(items.0[1].name, "Dictionary");
    }

    // ================================================================
    // 12. TypeAlias
    // ================================================================

    #[test]
    fn type_alias() {
        let (world, root, _) = build_from_source(
            "module Main\ntype IntPair = (Int64, Int64)",
        );

        let main = find_child_by_name(&world, root, &NodeKind::Module, "Main").unwrap();
        let alias = find_child_by_name(&world, main, &NodeKind::TypeAlias, "IntPair").unwrap();

        assert!(world.has::<Typed>(alias));
        assert!(world.get::<TypeAnnotation>(alias).is_some());
    }

    // ================================================================
    // 13. Subscript
    // ================================================================

    #[test]
    fn subscript_declaration() {
        let (world, root, _) = build_from_source(
            "module Main\nstruct Dict {\n  subscript(key: String) -> Int64 {\n    get {}\n    set {}\n  }\n}",
        );

        let main = find_child_by_name(&world, root, &NodeKind::Module, "Main").unwrap();
        let dict = find_child_by_name(&world, main, &NodeKind::Struct, "Dict").unwrap();
        let sub = find_child_by_kind(&world, dict, &NodeKind::Subscript).unwrap();

        assert!(world.has::<Subscript>(sub));
        assert!(world.has::<Gettable>(sub));
        assert!(world.has::<Settable>(sub));
        assert!(world.has::<Callable>(sub));
        assert!(world.get::<TypeAnnotation>(sub).is_some());
    }

    // ================================================================
    // 14. Attributes
    // ================================================================

    #[test]
    fn attributes_parsed() {
        let (world, root, _) = build_from_source(
            "module Main\n@inline\nfunc fast() {}",
        );

        let main = find_child_by_name(&world, root, &NodeKind::Module, "Main").unwrap();
        let func = find_child_by_name(&world, main, &NodeKind::Function, "fast").unwrap();

        let attrs = world.get::<Attributes>(func).unwrap();
        assert_eq!(attrs.0.len(), 1);
        assert_eq!(attrs.0[0].name, "inline");
    }

    // ================================================================
    // 15. FileId
    // ================================================================

    #[test]
    fn file_id_on_declarations_not_modules() {
        let (world, root, file) = build_from_source(
            "module Main\nstruct S {}",
        );

        let main = find_child_by_name(&world, root, &NodeKind::Module, "Main").unwrap();
        let s = find_child_by_name(&world, main, &NodeKind::Struct, "S").unwrap();

        assert!(world.get::<FileId>(main).is_none(), "module has no FileId");
        assert_eq!(world.get::<FileId>(s), Some(&FileId(file)));
    }

    // ================================================================
    // 16. AstType round-trip
    // ================================================================

    #[test]
    fn ast_type_round_trip() {
        let (world, root, _) = build_from_source(
            "module Main\nstruct S {\n  var a: Int64\n  var b: Array[String]\n  var c: (Int64, Bool)\n}",
        );

        let main = find_child_by_name(&world, root, &NodeKind::Module, "Main").unwrap();
        let s = find_child_by_name(&world, main, &NodeKind::Struct, "S").unwrap();

        let a = find_child_by_name(&world, s, &NodeKind::Field, "a").unwrap();
        let a_ty = world.get::<TypeAnnotation>(a).unwrap();
        match &a_ty.0 {
            AstType::Named { segments, .. } => {
                assert_eq!(segments.len(), 1);
                assert_eq!(segments[0].name, "Int64");
                assert!(segments[0].type_args.is_empty());
            }
            other => panic!("expected Named, got {:?}", other),
        }

        let b = find_child_by_name(&world, s, &NodeKind::Field, "b").unwrap();
        let b_ty = world.get::<TypeAnnotation>(b).unwrap();
        match &b_ty.0 {
            AstType::Named { segments, .. } => {
                assert_eq!(segments.len(), 1);
                assert_eq!(segments[0].name, "Array");
                assert_eq!(segments[0].type_args.len(), 1);
            }
            other => panic!("expected Named with type args, got {:?}", other),
        }

        let c = find_child_by_name(&world, s, &NodeKind::Field, "c").unwrap();
        let c_ty = world.get::<TypeAnnotation>(c).unwrap();
        match &c_ty.0 {
            AstType::Tuple(elems, _) => {
                assert_eq!(elems.len(), 2);
            }
            other => panic!("expected Tuple, got {:?}", other),
        }
    }

    // ================================================================
    // Initializer
    // ================================================================

    #[test]
    fn initializer() {
        let (world, root, _) = build_from_source(
            "module Main\nstruct S {\n  init(x: Int64) {}\n}",
        );

        let main = find_child_by_name(&world, root, &NodeKind::Module, "Main").unwrap();
        let s = find_child_by_name(&world, main, &NodeKind::Struct, "S").unwrap();
        let init = find_child_by_kind(&world, s, &NodeKind::Initializer).unwrap();

        let callable = world.get::<Callable>(init).unwrap();
        assert_eq!(callable.params.len(), 1);
        assert!(world.has::<Valued>(init));
    }

    // ================================================================
    // Deinit
    // ================================================================

    #[test]
    fn deinit_declaration() {
        let (world, root, _) = build_from_source(
            "module Main\nstruct S {\n  deinit {}\n}",
        );

        let main = find_child_by_name(&world, root, &NodeKind::Module, "Main").unwrap();
        let s = find_child_by_name(&world, main, &NodeKind::Struct, "S").unwrap();
        let deinit = find_child_by_kind(&world, s, &NodeKind::Deinit).unwrap();

        assert!(world.has::<Valued>(deinit));
        // Deinits now have Callable with consuming self receiver
        let callable = world.get::<Callable>(deinit).unwrap();
        assert_eq!(callable.receiver, Some(ReceiverKind::Consuming));
        assert!(callable.params.is_empty());
    }

    // ================================================================
    // Integration: real Kestrel source file
    // ================================================================

    #[test]
    fn ordering_ks_full_pipeline() {
        let source = include_str!("../../../lang/std/core/ordering.ks");
        let (world, root, file) = build_from_source(source);

        // Module hierarchy: std > core
        let std = find_child_by_name(&world, root, &NodeKind::Module, "std").unwrap();
        let core = find_child_by_name(&world, std, &NodeKind::Module, "core").unwrap();

        // Print the entity tree for visual inspection
        println!("\n=== ordering.ks entity tree ===\n");
        print_entity_tree(&world, root, 0, file);
        println!();

        // Verify import
        let import = find_child_by_kind(&world, core, &NodeKind::Import).unwrap();
        let path = world.get::<ModulePath>(import).unwrap();
        assert_eq!(path.0, vec!["std", "text"]);
        let items = world.get::<ImportItems>(import).unwrap();
        assert_eq!(items.0.len(), 3); // String, FormatOptions, Formattable

        // Verify enum Ordering
        let ordering = find_child_by_name(&world, core, &NodeKind::Enum, "Ordering").unwrap();
        assert!(world.has::<Typed>(ordering));
        assert_eq!(world.get::<Vis>(ordering), Some(&Vis::Public));
        assert_eq!(world.get::<FileId>(ordering), Some(&FileId(file)));

        // Conformances: Equatable, Formattable
        let conf = world.get::<Conformances>(ordering).unwrap();
        assert_eq!(conf.0.len(), 2);

        // Documentation — doc comments may not attach depending on CST trivia
        // placement, so we just check the enum itself
        if let Some(doc) = world.get::<Documentation>(ordering) {
            assert!(doc.0.contains("comparing"), "doc: {}", doc.0);
        }

        // Enum cases: Less, Equal, Greater
        let less = find_child_by_name(&world, ordering, &NodeKind::EnumCase, "Less").unwrap();
        let equal = find_child_by_name(&world, ordering, &NodeKind::EnumCase, "Equal").unwrap();
        let greater = find_child_by_name(&world, ordering, &NodeKind::EnumCase, "Greater").unwrap();
        assert!(!world.has::<Callable>(less), "plain case has no Callable");

        // Methods
        let equals_fn = find_child_by_name(&world, ordering, &NodeKind::Function, "equals").unwrap();
        let callable = world.get::<Callable>(equals_fn).unwrap();
        assert_eq!(callable.params.len(), 1);
        assert_eq!(callable.params[0].name, "other");
        assert!(world.has::<Valued>(equals_fn), "has body");
        assert_eq!(world.get::<Vis>(equals_fn), Some(&Vis::Public));
        // Return type
        let ret = world.get::<TypeAnnotation>(equals_fn).unwrap();
        match &ret.0 {
            AstType::Named { segments, .. } => {
                assert_eq!(segments.len(), 1);
                assert_eq!(segments[0].name, "Bool");
            }
            other => panic!("expected Named(Bool), got {:?}", other),
        }

        let reverse_fn = find_child_by_name(&world, ordering, &NodeKind::Function, "reverse").unwrap();
        let callable = world.get::<Callable>(reverse_fn).unwrap();
        assert_eq!(callable.params.len(), 0);

        let then_fn = find_child_by_name(&world, ordering, &NodeKind::Function, "then").unwrap();
        let callable = world.get::<Callable>(then_fn).unwrap();
        assert_eq!(callable.params.len(), 1);

        let then_with_fn = find_child_by_name(&world, ordering, &NodeKind::Function, "thenWith").unwrap();
        let callable = world.get::<Callable>(then_with_fn).unwrap();
        assert_eq!(callable.params.len(), 1);
        // Param type should be a function type
        let param_ty = callable.params[0].ty.as_ref().unwrap();
        assert!(matches!(param_ty, AstType::Function { .. }), "thenWith param should be Function type, got {:?}", param_ty);

        let format_fn = find_child_by_name(&world, ordering, &NodeKind::Function, "format").unwrap();
        let callable = world.get::<Callable>(format_fn).unwrap();
        assert_eq!(callable.params.len(), 1);
        assert!(callable.params[0].default_entity.is_some(), "format options has default value");

        // Total entity count
        let children = world.children_of(ordering);
        let case_count = children.iter().filter(|&&e| world.get::<NodeKind>(e) == Some(&NodeKind::EnumCase)).count();
        let fn_count = children.iter().filter(|&&e| world.get::<NodeKind>(e) == Some(&NodeKind::Function)).count();
        assert_eq!(case_count, 3, "3 enum cases");
        assert_eq!(fn_count, 6, "6 methods (equals, notEquals, reverse, then, thenWith, format)");
    }

    /// Pretty-print the entity tree for debugging.
    fn print_entity_tree(world: &World, entity: Entity, depth: usize, file: Entity) {
        let indent = "  ".repeat(depth);
        let kind = world.get::<NodeKind>(entity);
        let name = world.get::<Name>(entity);

        let kind_str = kind.map(|k| format!("{:?}", k)).unwrap_or_else(|| "???".into());
        let name_str = name.map(|n| format!(" \"{}\"", n.0)).unwrap_or_default();
        print!("{indent}{kind_str}{name_str}");

        let mut tags: Vec<String> = Vec::new();
        if world.has::<Typed>(entity) { tags.push("Typed".into()); }
        if world.has::<Gettable>(entity) { tags.push("Gettable".into()); }
        if world.has::<Settable>(entity) { tags.push("Settable".into()); }
        if world.has::<Static>(entity) { tags.push("Static".into()); }
        if world.has::<Subscript>(entity) { tags.push("Subscript".into()); }
        if world.has::<IsIndirect>(entity) { tags.push("IsIndirect".into()); }
        if world.has::<Valued>(entity) { tags.push("Valued".into()); }
        if let Some(vis) = world.get::<Vis>(entity) {
            tags.push(format!("{:?}", vis).to_lowercase());
        }
        if world.has::<FileId>(entity) { tags.push("FileId".into()); }
        if let Some(conf) = world.get::<Conformances>(entity) {
            let names: Vec<_> = conf.0.iter().map(|c| match c {
                ConformanceItem::Positive(ty, _) => format!("+{}", type_name(ty)),
                ConformanceItem::Negative(ty, _) => format!("-{}", type_name(ty)),
            }).collect();
            tags.push(format!("conforms({})", names.join(", ")));
        }
        if let Some(tp) = world.get::<TypeParams>(entity) {
            tags.push(format!("TypeParams({})", tp.0.len()));
        }
        if let Some(callable) = world.get::<Callable>(entity) {
            let params: Vec<_> = callable.params.iter().map(|p| {
                let ty_str = p.ty.as_ref().map(|t| format!(": {}", type_name(t))).unwrap_or_default();
                let label = p.label.as_ref().map(|l| format!("{} ", l)).unwrap_or_default();
                let dflt = if p.default_entity.is_some() { " = ..." } else { "" };
                format!("{label}{}{ty_str}{dflt}", p.name)
            }).collect();
            let recv = callable.receiver.as_ref().map(|r| format!("{:?} ", r)).unwrap_or_default();
            tags.push(format!("Callable({recv}({}))", params.join(", ")));
        }
        if let Some(ta) = world.get::<TypeAnnotation>(entity) {
            tags.push(format!("-> {}", type_name(&ta.0)));
        }
        if let Some(et) = world.get::<ExtensionTarget>(entity) {
            tags.push(format!("extends {}", type_name(&et.0)));
        }
        if let Some(mp) = world.get::<ModulePath>(entity) {
            tags.push(format!("path({})", mp.0.join(".")));
        }
        if let Some(items) = world.get::<ImportItems>(entity) {
            let names: Vec<_> = items.0.iter().map(|i| i.name.clone()).collect();
            tags.push(format!("items({})", names.join(", ")));
        }
        if let Some(doc) = world.get::<Documentation>(entity) {
            let first = doc.0.lines().next().unwrap_or("");
            let trunc = if first.len() > 50 { &first[..50] } else { first };
            tags.push(format!("/// {trunc}"));
        }
        if let Some(attrs) = world.get::<Attributes>(entity) {
            let names: Vec<_> = attrs.0.iter().map(|a| format!("@{}", a.name)).collect();
            tags.push(names.join(" "));
        }

        if !tags.is_empty() {
            print!("  [{}]", tags.join(", "));
        }
        println!();

        for &child in world.children_of(entity) {
            print_entity_tree(world, child, depth + 1, file);
        }
    }

    /// Short display name for an AstType.
    fn type_name(ty: &AstType) -> String {
        match ty {
            AstType::Named { segments, .. } => {
                let parts: Vec<_> = segments.iter().map(|seg| {
                    if seg.type_args.is_empty() {
                        seg.name.clone()
                    } else {
                        let args: Vec<_> = seg.type_args.iter().map(type_name).collect();
                        format!("{}[{}]", seg.name, args.join(", "))
                    }
                }).collect();
                parts.join(".")
            }
            AstType::Tuple(elems, _) => {
                let inner: Vec<_> = elems.iter().map(type_name).collect();
                format!("({})", inner.join(", "))
            }
            AstType::Function { params, return_type, .. } => {
                let p: Vec<_> = params.iter().map(type_name).collect();
                format!("({}) -> {}", p.join(", "), type_name(return_type))
            }
            AstType::Array(inner, _) => format!("[{}]", type_name(inner)),
            AstType::Dictionary(k, v, _) => format!("[{}: {}]", type_name(k), type_name(v)),
            AstType::Optional(inner, _) => format!("{}?", type_name(inner)),
            AstType::Result { ok, err, .. } => format!("{}!{}", type_name(ok), type_name(err)),
            AstType::Unit(_) => "()".into(),
            AstType::Never(_) => "Never".into(),
            AstType::Inferred(_) => "_".into(),
        }
    }
}
