#[cfg(test)]
mod tests {
    use kestrel_compiler::Compiler;

    #[test]
    fn signature_keeps_trailing_bracket_in_return_type() {
        let src = "module T\npublic enum Result[T, E] {\n  public func err() -> Optional[E] { .None }\n}\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/x2.ks", src.into());
        c.build(f);

        let (_, pages) = crate::extract(c.world(), c.root());
        let page = pages.into_iter().find(|p| p.path == "T").expect("page");
        let result = page
            .items
            .iter()
            .find(|it| it.name == "Result")
            .expect("Result enum");
        let direct = result
            .member_groups
            .iter()
            .find(|g| g.kind == "direct")
            .expect("direct group");
        let err = direct
            .members
            .iter()
            .find(|m| m.name == "err")
            .expect("err method");
        assert!(
            err.signature.contains("Optional[E]"),
            "signature should preserve trailing bracket: {:?}",
            err.signature
        );
    }

    #[test]
    fn extract_filters_private_members() {
        let src = "module T\npublic struct S {\n  private var x: Int64\n  public var y: Int64\n}\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/v2.ks", src.into());
        c.build(f);

        let (_, pages) = crate::extract(c.world(), c.root());
        let page = pages.into_iter().find(|p| p.path == "T").expect("page");
        let s = page.items.iter().find(|it| it.name == "S").expect("S");
        let direct = s
            .member_groups
            .iter()
            .find(|g| g.kind == "direct")
            .expect("direct group");
        let names: Vec<_> = direct.members.iter().map(|m| m.name.clone()).collect();
        assert!(names.contains(&"y".to_string()));
        assert!(!names.contains(&"x".to_string()), "private leaked: {:?}", names);
    }

    #[test]
    fn signature_renders_subscript_with_get_set() {
        let src = "module T\npublic struct S {\n  public subscript(i: Int64) -> Int64 { get { 0 } set { } }\n}\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/sub.ks", src.into());
        c.build(f);

        let (_, pages) = crate::extract(c.world(), c.root());
        let page = pages.into_iter().find(|p| p.path == "T").unwrap();
        let s = page.items.iter().find(|it| it.name == "S").unwrap();
        let direct = s.member_groups.iter().find(|g| g.kind == "direct").unwrap();
        let sub = direct
            .members
            .iter()
            .find(|m| m.kind == "subscript")
            .expect("subscript");
        assert!(sub.signature.contains("{ get set }"), "got: {}", sub.signature);
    }

    #[test]
    fn signature_hides_bind_names() {
        // `func foo(label name: T)` should render as `foo(label: T)` —
        // the internal `name` is dropped in the docs view.
        let src = "module T\npublic func foo(label name: Int64) { }\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/hb.ks", src.into());
        c.build(f);
        let (_, pages) = crate::extract(c.world(), c.root());
        let page = pages.into_iter().find(|p| p.path == "T").unwrap();
        let foo = page.items.iter().find(|it| it.name == "foo").unwrap();
        assert!(
            foo.signature.contains("(label: Int64)"),
            "signature: {}",
            foo.signature
        );
        assert!(
            !foo.signature.contains("label name"),
            "internal name leaked: {}",
            foo.signature
        );
    }

    #[test]
    fn protocol_implementation_routed_to_protocol_group() {
        // Foo implements Eq's `eq`. The implementation should land
        // under the `Eq` protocol group, not in `direct`. A separate
        // method on Foo (`extra`) that doesn't match a protocol member
        // stays in `direct`.
        let src = r#"
module T
public protocol Eq {
  func eq(other: Self) -> Bool
}
public struct Foo: Eq {
  public func eq(other: Self) -> Bool { true }
  public func extra() -> Bool { true }
}
"#;
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/proto.ks", src.into());
        c.build(f);
        let (_, pages) = crate::extract(c.world(), c.root());
        let page = pages.into_iter().find(|p| p.path == "T").unwrap();
        let foo = page.items.iter().find(|it| it.name == "Foo").unwrap();

        let direct = foo
            .member_groups
            .iter()
            .find(|g| g.kind == "direct")
            .expect("direct group");
        let direct_names: Vec<_> = direct.members.iter().map(|m| m.name.clone()).collect();
        assert_eq!(direct_names, vec!["extra".to_string()]);

        let proto_group = foo
            .member_groups
            .iter()
            .find(|g| g.kind == "protocol" && g.label.as_deref() == Some("Eq"))
            .expect("Eq group");
        assert!(proto_group.members.iter().any(|m| m.name == "eq"));
    }

    #[test]
    fn extension_members_merged_into_target() {
        // An `extend Foo: Eq { func eq... }` should not appear as a
        // standalone item — its members get pulled into Foo's docs and
        // its conformance shows up as an "Implements Eq" group.
        let src = r#"
module T
public protocol Eq {
  func eq(other: Self) -> Bool
}
public struct Foo {
  public func native() -> Bool { true }
}
extend Foo: Eq {
  public func eq(other: Self) -> Bool { true }
}
"#;
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/ext.ks", src.into());
        c.build(f);
        let (_, pages) = crate::extract(c.world(), c.root());
        let page = pages.into_iter().find(|p| p.path == "T").unwrap();

        // Extension is not its own item.
        assert!(
            page.items.iter().all(|it| it.kind != "extension"),
            "extension leaked as standalone item: {:?}",
            page.items.iter().map(|it| &it.name).collect::<Vec<_>>()
        );

        let foo = page.items.iter().find(|it| it.name == "Foo").unwrap();

        // Foo's own method stays in `direct`.
        let direct = foo
            .member_groups
            .iter()
            .find(|g| g.kind == "direct")
            .expect("direct group");
        assert!(direct.members.iter().any(|m| m.name == "native"));

        // The extension-supplied `eq` is routed under "Implements Eq".
        let eq_group = foo
            .member_groups
            .iter()
            .find(|g| g.kind == "protocol" && g.label.as_deref() == Some("Eq"))
            .expect("Eq group from extension");
        assert!(eq_group.members.iter().any(|m| m.name == "eq"));
    }

    #[test]
    fn stdlib_array_no_private_leaks() {
        // Regression: the parser used to produce Error tokens around
        // accessor-block braces, which cascaded into the next decl's
        // prelude — sometimes resulting in a missing `Vis` component on
        // `private` methods/fields, sometimes losing their doc comment.
        // Now fixed at the parser layer, but we keep this end-to-end
        // test to make sure no private items leak into Array's docs.
        let mut c = Compiler::new();
        c.load_dir(std::path::Path::new("../../lang/std"));
        let world = c.world();

        let (_, pages) = crate::extract(world, c.root());
        let collections = pages
            .into_iter()
            .find(|p| p.path == "std.collections")
            .expect("std.collections page");
        let array = collections
            .items
            .iter()
            .find(|it| it.name == "Array")
            .expect("Array struct");
        for group in &array.member_groups {
            for m in &group.members {
                assert!(
                    !m.signature.starts_with("private")
                        && !m.signature.starts_with("fileprivate"),
                    "private leaked through ({}): {}",
                    group.kind,
                    m.signature
                );
            }
        }
    }
}

#[cfg(test)]
mod multi_subscript_diag {
    use kestrel_ast_builder::{Documentation, Name, NodeKind};
    use kestrel_compiler::Compiler;

    #[test]
    fn subsequent_subscripts_have_docs() {
        let mut c = Compiler::new();
        c.load_dir(std::path::Path::new("../../lang/std"));
        let world = c.world();

        let mut count = 0;
        for (e, n) in world.iter_component::<Name>() {
            if n.0 != "subscript" {
                continue;
            }
            if !matches!(world.get::<NodeKind>(e), Some(NodeKind::Subscript)) {
                continue;
            }
            let parent = world.parent_of(e);
            let parent_name = parent.and_then(|p| world.get::<Name>(p).map(|n| n.0.clone()));
            if parent_name.as_deref() != Some("Array") {
                continue;
            }
            let docs = world.get::<Documentation>(e).map(|d| d.0.clone());
            count += 1;
            eprintln!(
                "Array subscript #{}: docs={:?}",
                count,
                docs.as_deref().map(|s| &s[0..s.len().min(60)])
            );
        }
    }
}
