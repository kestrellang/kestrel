//! Operator precedence (decided by the parser, audit H6) and interpolated
//! strings (modal lexing, holes parsed in place, audit H4).

mod common;

use common::{first, parse, parse_ok, shape};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

/// The operator tree of the outermost binary expression in a function body,
/// fully parenthesised.
fn ops(expr: &str) -> String {
    let tree = parse_ok(&format!("func f() {{ {expr} }}\n"));
    let binary = first(&tree, SyntaxKind::ExprBinary);
    show(&binary.parent().unwrap())
}

fn show(n: &SyntaxNode) -> String {
    let inner = if n.kind() == SyntaxKind::Expression {
        n.first_child().unwrap()
    } else {
        n.clone()
    };
    if inner.kind() != SyntaxKind::ExprBinary {
        return inner.text().to_string().trim().to_string();
    }
    let kids: Vec<_> = inner.children().collect();
    let op = inner
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| !t.kind().is_trivia())
        .unwrap();
    format!("({} {} {})", show(&kids[0]), op.text(), show(&kids[1]))
}

#[test]
fn binary_precedence_is_in_the_tree() {
    assert_eq!(ops("a + b * c"), "(a + (b * c))");
    assert_eq!(ops("a * b + c"), "((a * b) + c)");
    assert_eq!(ops("a - b - c"), "((a - b) - c)");
    assert_eq!(ops("a ?? b ?? c"), "(a ?? (b ?? c))");
    assert_eq!(
        ops("a or b and c == d + e * f << g"),
        "(a or (b and (c == (d + (e * (f << g))))))"
    );
    // Comparisons do not chain specially: left-associative, as before.
    assert_eq!(ops("a < b < c"), "((a < b) < c)");
    assert_eq!(ops("0..<n + 1"), "(0 ..< (n + 1))");
    assert_eq!(ops("x & y | z"), "((x & y) | z)");
    assert_eq!(ops("-a * b"), "(-a * b)");
    assert_eq!(ops("try f() ?? d"), "(try f() ?? d)");
}

#[test]
fn binary_spans_cover_their_operands() {
    let tree = parse_ok("func f() { a + b * c }\n");
    let spans: Vec<_> = tree
        .descendants()
        .filter(|n| n.kind() == SyntaxKind::ExprBinary)
        .map(|n| n.text().to_string().trim().to_string())
        .collect();
    assert_eq!(spans, vec!["a + b * c", "b * c"]);
}

#[test]
fn interpolated_string_holes_are_nodes() {
    let tree = parse_ok("func f() { \"a \\(x + 1) b \\(y:08x)\" }\n");
    let s = first(&tree, SyntaxKind::ExprInterpolatedString);
    assert_eq!(
        shape(&s),
        "ExprInterpolatedString(\" a  StringInterpolation(\\( Expression(ExprBinary(\
         Expression(ExprPath(x)) + Expression(ExprInteger(1)))) ))  b  StringInterpolation(\
         \\( Expression(ExprPath(y)) FormatSpecifier(: 08x) )) \")"
    );
}

#[test]
fn plain_string_is_one_token() {
    let tree = parse_ok("func f() { \"a \\\\(b) c\" }\n");
    let s = first(&tree, SyntaxKind::ExprString);
    let tokens: Vec<_> = s
        .children_with_tokens()
        .filter(|e| !e.kind().is_trivia())
        .map(|e| e.kind())
        .collect();
    assert_eq!(tokens, vec![SyntaxKind::String]);
}

#[test]
fn dictionary_colon_in_a_hole_is_not_a_format_spec() {
    let tree = parse_ok("func f() { \"\\([1: 2].count)\" }\n");
    assert!(
        tree.descendants()
            .all(|n| n.kind() != SyntaxKind::FormatSpecifier)
    );
    assert_eq!(
        tree.descendants()
            .filter(|n| n.kind() == SyntaxKind::ExprDictionary)
            .count(),
        1
    );
}

#[test]
fn nested_interpolation_and_it_in_holes() {
    let tree = parse_ok("func f() { xs.map { \"v \\(\"in \\(it)\")\" } }\n");
    let its = tree
        .descendants_with_tokens()
        .filter(|e| e.as_token().is_some_and(|t| t.text() == "it"))
        .count();
    assert_eq!(its, 1);
    assert_eq!(
        tree.descendants()
            .filter(|n| n.kind() == SyntaxKind::StringInterpolation)
            .count(),
        2
    );
}

#[test]
fn broken_hole_is_an_error_node_with_an_interpolation_error() {
    let result = parse("func f() { let _ = \"v=\\(n.0.0)\"; }\n");
    assert!(!result.errors.is_empty());
    for e in &result.errors {
        assert!(
            e.message
                .starts_with("invalid expression in string interpolation"),
            "{e:?}"
        );
    }
    let hole = first(&result.tree, SyntaxKind::StringInterpolation);
    assert!(hole.children().any(|n| n.kind() == SyntaxKind::Error));
}

#[test]
fn unterminated_string_does_not_swallow_the_file() {
    let result = parse("func f() { let s = \"abc\n}\nfunc g() {}\n");
    assert_eq!(
        result
            .tree
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::FunctionDeclaration)
            .count(),
        2
    );
}
