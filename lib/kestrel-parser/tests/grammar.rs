//! Grammar-level tests for the handwritten parser: tree shapes downstream
//! code relies on, the no-`Error`-on-valid-input invariant, losslessness,
//! and linear time on nesting that used to be exponential.

mod common;

use common::{first, parse, parse_ok, shape};
use kestrel_syntax_tree::SyntaxKind;

#[test]
fn separators_and_brackets_are_tokens_not_error_gaps() {
    let tree = parse_ok("func f(a: Int64, b: Box[Int64, Bool]) {}\n");
    let params = first(&tree, SyntaxKind::ParameterList);
    let commas = params
        .children_with_tokens()
        .filter(|e| e.kind() == SyntaxKind::Comma)
        .count();
    assert_eq!(commas, 1);
    let args = first(&tree, SyntaxKind::TypeArgumentList);
    assert_eq!(
        shape(&args),
        "TypeArgumentList([ Ty(TyPath(Path(PathElement(Int64)))) , Ty(TyPath(Path(PathElement(Bool)))) ])"
    );
}

#[test]
fn trailing_closure_follows_the_closing_paren() {
    let tree = parse_ok("func f() { foo(1, 2) { (x) in x }; }\n");
    let args = first(&tree, SyntaxKind::ArgumentList);
    let kinds: Vec<_> = args
        .children_with_tokens()
        .filter(|e| !e.kind().is_trivia())
        .map(|e| e.kind())
        .collect();
    assert_eq!(
        kinds,
        vec![
            SyntaxKind::LParen,
            SyntaxKind::Argument,
            SyntaxKind::Comma,
            SyntaxKind::Argument,
            SyntaxKind::RParen,
            SyntaxKind::Argument,
        ]
    );
}

#[test]
fn trailing_closure_without_parens_makes_a_call() {
    let tree = parse_ok("func f() { xs.map { it * 2 } }\n");
    let call = first(&tree, SyntaxKind::ExprCall);
    assert_eq!(
        shape(&call).split(" ArgumentList").next().unwrap(),
        "ExprCall(Expression(ExprPath(xs . map))"
    );
}

#[test]
fn trailing_closure_must_start_on_the_same_line() {
    // `{ 1 }` on its own line is not an argument of `foo` — so `foo()` is
    // a statement missing its `;`.
    let result = parse("func f() {\n    foo()\n    { 1 }\n}\n");
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.errors[0].code, Some("E801"));
    let tree = result.tree;
    let call = first(&tree, SyntaxKind::ExprCall);
    assert_eq!(
        call.descendants()
            .filter(|n| n.kind() == SyntaxKind::Argument)
            .count(),
        0
    );
}

#[test]
fn conditions_take_no_trailing_closure() {
    let tree = parse_ok("func f() { if x { y } }\n");
    let cond = first(&tree, SyntaxKind::ExprIf);
    assert!(cond.descendants().all(|n| n.kind() != SyntaxKind::ExprCall));
}

#[test]
fn member_access_extends_the_path() {
    let tree = parse_ok("func f() { a.b().c.d }\n");
    let block = first(&tree, SyntaxKind::CodeBlock);
    assert_eq!(
        shape(&block),
        "CodeBlock({ Expression(ExprPath(Expression(ExprCall(Expression(ExprPath(a . b)) \
         ArgumentList(( )))) . c . d)) })"
    );
}

#[test]
fn statement_like_expressions_need_no_semicolon() {
    let tree = parse_ok("func f() { if a { b(); } c(); while x { } d() }\n");
    let block = first(&tree, SyntaxKind::CodeBlock);
    let statements = block
        .children()
        .filter(|n| n.kind() == SyntaxKind::Statement)
        .count();
    assert_eq!(statements, 3, "{}", shape(&block));
}

#[test]
fn function_body_trailing_return_is_the_value_but_inline_it_is_a_statement() {
    let tree = parse_ok("func f() -> Int64 { if c { return 1 } return 2 }\n");
    let body = first(&tree, SyntaxKind::FunctionBody);
    let outer = body.first_child().unwrap();
    // Declaration body: the last `return` is the block's value.
    assert_eq!(outer.last_child().unwrap().kind(), SyntaxKind::Expression);
    // `if` body: `return 1` stands as a statement.
    let inner = first(&first(&tree, SyntaxKind::ExprIf), SyntaxKind::CodeBlock);
    assert_eq!(inner.first_child().unwrap().kind(), SyntaxKind::Statement);
}

#[test]
fn closure_header_needs_in() {
    let with = parse_ok("func f() { g { (a, b) in a } }\n");
    assert_eq!(
        first(&with, SyntaxKind::ClosureParams).children().count(),
        2
    );
    let without = parse_ok("func f() { g { (a, b) } }\n");
    assert!(
        without
            .descendants()
            .all(|n| n.kind() != SyntaxKind::ClosureParams)
    );
}

#[test]
fn qualified_associated_type_binding() {
    let tree = parse_ok("extend X: P { type P[Int64].Item = Bool; }\n");
    let target = first(&tree, SyntaxKind::AssociatedTypeTarget);
    assert_eq!(
        shape(&target),
        "AssociatedTypeTarget(Ty(TyPath(Path(PathElement(P)) TypeArgumentList([ \
         Ty(TyPath(Path(PathElement(Int64)))) ]))) . Name(Item))"
    );
}

#[test]
fn where_clause_bounds_keep_their_colon_and_and() {
    let tree = parse_ok("func f[T]() where T: A and B[C], T.Item = Int64 {}\n");
    let clause = first(&tree, SyntaxKind::WhereClause);
    assert_eq!(
        shape(&clause),
        "WhereClause(where TypeBound(Name(T) : Path(PathElement(A)) and Path(PathElement(B)) \
         TypeArgumentList([ Ty(TyPath(Path(PathElement(C)))) ])) , TypeEquality(\
         AssociatedTypeTarget(Path(PathElement(T) . PathElement(Item))) = \
         Ty(TyPath(Path(PathElement(Int64))))))"
    );
}

#[test]
fn double_optional_is_one_token() {
    let tree = parse_ok("let x: Int64?? = .None;\n");
    let ty = first(&tree, SyntaxKind::Ty);
    assert_eq!(
        shape(&ty),
        "Ty(TyOptional(Ty(TyOptional(Ty(TyPath(Path(PathElement(Int64)))))) ??))"
    );
}

#[test]
fn enum_pattern_args_are_labels_or_patterns() {
    parse_ok("func f() { match o { .A(x, y: .B(_)) => 1, .C((a, b)) => 2, _ => 3 } }\n");
    // A bare identifier is a label, so it cannot carry `@` or `or`.
    assert!(
        !parse("func f() { match o { .A(x @ 5) => 1 } }\n")
            .errors
            .is_empty()
    );
}

#[test]
fn every_error_has_a_code() {
    for src in [
        "func f( {",
        "struct S { var x: ; }",
        "func f() { let x = ; foo( }",
        "module",
        "@ @ @",
        "func f() { a. }",
    ] {
        let result = parse(src);
        assert!(!result.errors.is_empty(), "{src}");
        for e in &result.errors {
            assert!(
                e.code.is_some_and(|c| c.starts_with("E8")),
                "uncoded error {e:?} for {src}"
            );
        }
        assert_eq!(result.tree.text().to_string(), src);
    }
}

#[test]
fn errors_are_sorted_and_deduplicated() {
    let result = parse("func f() { let x = 1 let y = 2 }\nfunc g( {\n");
    let starts: Vec<_> = result
        .errors
        .iter()
        .map(|e| e.span.as_ref().unwrap().start)
        .collect();
    let mut sorted = starts.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(starts, sorted);
}

#[test]
fn struct_body_recovers_per_member() {
    let result = parse("struct S {\n    var a: Int64;\n    ???\n    var b: Int64;\n}\n");
    assert_eq!(result.errors.len(), 1, "{:?}", result.errors);
    let fields = result
        .tree
        .descendants()
        .filter(|n| n.kind() == SyntaxKind::FieldDeclaration)
        .count();
    assert_eq!(fields, 2);
}

#[test]
fn nested_trailing_closures_parse_in_linear_time() {
    // Depth 20 took ~33 s with the combinator parser (tail expressions were
    // parsed twice per level). Now it must be instant.
    let mut body = String::from("x()");
    for _ in 0..200 {
        body = format!("f {{ {body} }}");
    }
    let source = format!("func main() {{\n    {body}\n}}\n");
    let start = std::time::Instant::now();
    parse_ok(&source);
    assert!(start.elapsed().as_secs() < 5, "took {:?}", start.elapsed());
}

#[test]
fn stdlib_parses_without_errors_or_error_nodes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lang/std");
    let mut files = 0;
    for entry in walk(&root) {
        let source = std::fs::read_to_string(&entry).unwrap();
        let result = parse(&source);
        assert!(
            result.errors.is_empty(),
            "{}: {:?}",
            entry.display(),
            result.errors
        );
        assert_eq!(
            result.tree.text().to_string(),
            source,
            "{}",
            entry.display()
        );
        assert!(
            result
                .tree
                .descendants_with_tokens()
                .all(|e| e.kind() != SyntaxKind::Error),
            "{} has Error elements",
            entry.display()
        );
        files += 1;
    }
    assert!(files > 50, "stdlib not found at {}", root.display());
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else if path.extension().is_some_and(|e| e == "ks") {
            out.push(path);
        }
    }
    out
}
