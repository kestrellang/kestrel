//! Tuple codegen tests.

use kestrel_test_suite::*;

#[test]
fn test_tuple_construction() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let t = (42, 0);
    if t.0 != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_tuple_second_element() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let t = (0, 42);
    if t.1 != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_tuple_multiple_elements() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let t: (std.num.Int64, std.num.Int64, std.num.Int64) = (10, 20, 12);
    if t.0 + t.1 + t.2 != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_tuple_mixed_types() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let t = (true, 42);
    if t.1 != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_nested_tuple() {
    // Note: t.0.0 parses as t.(0.0) which is a float literal
    // So we use a temporary variable to work around this parser limitation
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let t: ((std.num.Int64, std.num.Int64), std.num.Int64) = ((40, 2), 0);
    let inner = t.0;
    if inner.0 + inner.1 != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_tuple_from_function() {
    Test::new(
        r#"module Test

func make_pair(a: std.num.Int64, b: std.num.Int64) -> (std.num.Int64, std.num.Int64) {
    (a, b)
}

func main() -> lang.i64 {
    let t = make_pair(20, 22);
    if t.0 + t.1 != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
