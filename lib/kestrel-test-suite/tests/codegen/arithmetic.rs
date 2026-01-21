//! Arithmetic operation tests.

use kestrel_test_suite::*;

#[test]
fn test_return_constant() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x = 42;
    if x != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_add() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 10;
    let y: std.num.Int64 = 32;
    if x + y != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_subtract() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 50;
    let y: std.num.Int64 = 8;
    if x - y != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_multiply() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 6;
    let y: std.num.Int64 = 7;
    if x * y != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_divide() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 84;
    let y: std.num.Int64 = 2;
    if x / y != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_modulo() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 142;
    let y: std.num.Int64 = 100;
    if x % y != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_negation() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = -42;
    if -x != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_complex_expression() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let a: std.num.Int64 = 10;
    let b: std.num.Int64 = 3;
    let c: std.num.Int64 = 2;
    // (10 + 3) * 2 + (10 - 3) = 13 * 2 + 7 = 26 + 7 = 33
    if (a + b) * c + (a - b) != 33 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_return_zero() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_unit_main() {
    Test::new(
        r#"module Test

func main() {
    let x: std.num.Int64 = 42;
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
