//! Control flow tests (if/else, comparisons, boolean operations).

use kestrel_test_suite::*;

// =============================================================================
// Simple if-else tests
// =============================================================================

#[test]
fn test_if_else_true_branch() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    if true {
        0
    } else {
        1
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_if_else_false_branch() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    if false {
        1
    } else {
        0
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_if_else_with_comparison() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 10;
    if x > 5 {
        0
    } else {
        1
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_guard_let_optional_type_operator() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let opt: std.num.Int64? = .Some(1);
    guard let .Some(v) = opt else {
        return 1
    }
    if v != 1 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Comparison operator tests
// =============================================================================

#[test]
fn test_equal_true() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 5;
    if x == 5 {
        0
    } else {
        1
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_equal_false() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 5;
    if x == 10 {
        1
    } else {
        0
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_not_equal_true() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 5;
    if x != 10 {
        0
    } else {
        1
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_less_than_true() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 3;
    if x < 5 {
        0
    } else {
        1
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_less_than_false() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 10;
    if x < 5 {
        1
    } else {
        0
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_greater_than_true() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 10;
    if x > 5 {
        0
    } else {
        1
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_less_than_or_equal_equal() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 5;
    if x <= 5 {
        0
    } else {
        1
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_less_than_or_equal_less() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 3;
    if x <= 5 {
        0
    } else {
        1
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_greater_than_or_equal_equal() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 5;
    if x >= 5 {
        0
    } else {
        1
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_greater_than_or_equal_greater() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 10;
    if x >= 5 {
        0
    } else {
        1
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Boolean operation tests
// =============================================================================

#[test]
fn test_bool_and_true() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let a: std.core.Bool = true;
    let b: std.core.Bool = true;
    if a and b {
        0
    } else {
        1
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_bool_and_false() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let a: std.core.Bool = true;
    let b: std.core.Bool = false;
    if a and b {
        1
    } else {
        0
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_bool_or_true() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let a: std.core.Bool = false;
    let b: std.core.Bool = true;
    if a or b {
        0
    } else {
        1
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_bool_or_false() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let a: std.core.Bool = false;
    let b: std.core.Bool = false;
    if a or b {
        1
    } else {
        0
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_bool_not_true() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    if not false {
        0
    } else {
        1
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_bool_not_false() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    if not true {
        1
    } else {
        0
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Nested if-else tests
// =============================================================================

#[test]
fn test_nested_if_else() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 10;
    if x > 5 {
        if x > 15 {
            1
        } else {
            0
        }
    } else {
        1
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_if_else_chain() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 10;
    if x < 5 {
        1
    } else if x < 15 {
        0
    } else {
        1
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Complex expressions
// =============================================================================

#[test]
fn test_comparison_in_expression() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let a: std.num.Int64 = 5;
    let b: std.num.Int64 = 10;
    if (a < b) and (b > 5) {
        0
    } else {
        1
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_variable_from_branch() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 10;
    let y: std.num.Int64 = if x > 5 { 40 } else { 0 };
    if y + 2 != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
