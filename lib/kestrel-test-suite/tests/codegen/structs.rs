//! Struct construction and field access tests.

use super::compile_and_run;

#[test]
#[ignore]
fn test_struct_construction() {
    let result = compile_and_run(
        r#"
module Test

struct Point {
    let x: lang.i64
    let y: lang.i64
}

func main() -> lang.i64 {
    let p = Point(x: 42, y: 0);
    p.x
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore]
fn test_struct_field_access() {
    let result = compile_and_run(
        r#"
module Test

struct Point {
    let x: lang.i64
    let y: lang.i64
}

func main() -> lang.i64 {
    let p = Point(x: 10, y: 32);
    p.x + p.y
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore]
fn test_struct_multiple_fields() {
    let result = compile_and_run(
        r#"
module Test

struct Data {
    let a: lang.i64
    let b: lang.i64
    let c: lang.i64
}

func main() -> lang.i64 {
    let d = Data(a: 10, b: 20, c: 12);
    d.a + d.b + d.c
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore]
fn test_struct_pass_to_function() {
    let result = compile_and_run(
        r#"
module Test

struct Point {
    let x: lang.i64
    let y: lang.i64
}

func sum_point(p: Point) -> lang.i64 {
    p.x + p.y
}

func main() -> lang.i64 {
    let p = Point(x: 20, y: 22);
    sum_point(p)
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore]
fn test_struct_return_from_function() {
    let result = compile_and_run(
        r#"
module Test

struct Point {
    let x: lang.i64
    let y: lang.i64
}

func make_point(x: lang.i64, y: lang.i64) -> Point {
    Point(x: x, y: y)
}

func main() -> lang.i64 {
    let p = make_point(10, 32);
    p.x + p.y
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore]
fn test_nested_struct_extra() {
    // Test accessing a field after a nested struct field
    let result = compile_and_run(
        r#"
module Test

struct Inner {
    let value: lang.i64
}

struct Outer {
    let inner: Inner
    let extra: lang.i64
}

func main() -> lang.i64 {
    let o = Outer(inner: Inner(value: 40), extra: 42);
    o.extra
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore]
fn test_nested_struct_inner_value() {
    // Test accessing a field inside a nested struct
    let result = compile_and_run(
        r#"
module Test

struct Inner {
    let value: lang.i64
}

struct Outer {
    let inner: Inner
    let extra: lang.i64
}

func main() -> lang.i64 {
    let o = Outer(inner: Inner(value: 42), extra: 0);
    o.inner.value
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore]
fn test_nested_struct() {
    let result = compile_and_run(
        r#"
module Test

struct Inner {
    let value: lang.i64
}

struct Outer {
    let inner: Inner
    let extra: lang.i64
}

func main() -> lang.i64 {
    let o = Outer(inner: Inner(value: 40), extra: 2);
    o.inner.value + o.extra
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore]
fn test_struct_second_field() {
    let result = compile_and_run(
        r#"
module Test

struct Point {
    let x: lang.i64
    let y: lang.i64
}

func main() -> lang.i64 {
    let p = Point(x: 0, y: 42);
    p.y
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}
