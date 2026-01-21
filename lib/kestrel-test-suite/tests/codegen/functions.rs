//! Function call tests.

use kestrel_test_suite::*;

#[test]
fn test_call_simple_function() {
    Test::new(
        r#"module Test

func add(a: std.num.Int64, b: std.num.Int64) -> std.num.Int64 {
    a + b
}

func main() -> lang.i64 {
    if add(20, 22) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_call_function_no_args() {
    Test::new(
        r#"module Test

func get_answer() -> std.num.Int64 {
    42
}

func main() -> lang.i64 {
    if get_answer() != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_call_function_chain() {
    Test::new(
        r#"module Test

func double(x: std.num.Int64) -> std.num.Int64 {
    x * 2
}

func add_ten(x: std.num.Int64) -> std.num.Int64 {
    x + 10
}

func main() -> lang.i64 {
    // double(16) = 32, add_ten(32) = 42
    if add_ten(double(16)) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_call_unit_function() {
    Test::new(
        r#"module Test

func do_nothing() {
}

func main() -> lang.i64 {
    do_nothing();
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_call_multiple_functions() {
    Test::new(
        r#"module Test

func mul(a: std.num.Int64, b: std.num.Int64) -> std.num.Int64 {
    a * b
}

func add(a: std.num.Int64, b: std.num.Int64) -> std.num.Int64 {
    a + b
}

func main() -> lang.i64 {
    if add(mul(6, 7), 0) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_call_with_local_variables() {
    Test::new(
        r#"module Test

func square(x: std.num.Int64) -> std.num.Int64 {
    x * x
}

func main() -> lang.i64 {
    let a: std.num.Int64 = 6;
    let b = square(a);
    // square(6) = 36, 36 + 6 = 42
    if b + 6 != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_nested_function_calls() {
    Test::new(
        r#"module Test

func add(a: std.num.Int64, b: std.num.Int64) -> std.num.Int64 {
    a + b
}

func main() -> lang.i64 {
    // add(10, 12) = 22, add(10, 10) = 20, add(22, 20) = 42
    if add(add(10, 12), add(10, 10)) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_recursive_factorial() {
    Test::new(
        r#"module Test

func factorial(n: std.num.Int64) -> std.num.Int64 {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}

func main() -> lang.i64 {
    // 5! = 120
    if factorial(5) != 120 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
