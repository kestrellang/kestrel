#[cfg(test)]
mod tests {
    use kestrel_ast_builder::{CstNode, DeclSpan, Name, NodeKind};
    use kestrel_compiler::Compiler;

    #[test]
    fn body_block_offset_for_bracket_return() {
        let src = "module T\npublic enum Result[T, E] {\n  public func err() -> Optional[E] { .None }\n}\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/x.ks", src.into());
        c.build(f);

        let world = c.world();
        for (e, n) in world.iter_component::<Name>() {
            if n.0 != "err" {
                continue;
            }
            let kind = world.get::<NodeKind>(e).unwrap();
            let cst = world.get::<CstNode>(e).unwrap();
            let span = world.get::<DeclSpan>(e).unwrap();
            eprintln!("kind={:?} decl_span={}..{}", kind, span.0.start, span.0.end);
            eprintln!("cst kind={:?} range={:?}", cst.0.kind(), cst.0.text_range());
            eprintln!("cst text={:?}", cst.0.text().to_string());
            for child in cst.0.children() {
                eprintln!(
                    "  child {:?} @ {:?} text={:?}",
                    child.kind(),
                    child.text_range(),
                    child.text().to_string()
                );
            }
            let sliced = &src[span.0.start..span.0.end.min(src.len())];
            eprintln!("decl-span slice: {:?}", sliced);
        }
    }

    #[test]
    fn signature_keeps_trailing_bracket_in_return_type() {
        let src = "module T\npublic enum Result[T, E] {\n  public func err() -> Optional[E] { .None }\n}\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/x2.ks", src.into());
        c.build(f);

        let world = c.world();
        let (page, _) = {
            let (idx, pages) = crate::extract(world, c.root());
            let page = pages
                .into_iter()
                .find(|p| p.path == "T")
                .expect("T module page");
            (page, idx)
        };
        let result = page
            .items
            .iter()
            .find(|it| it.name == "Result")
            .expect("Result enum");
        let err = result
            .members
            .iter()
            .find(|m| m.name == "err")
            .expect("err method");
        assert!(
            err.signature.ends_with("Optional[E]"),
            "signature should end with the closing bracket: {:?}",
            err.signature
        );
    }
}
