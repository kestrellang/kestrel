//! Control flow tests (if/else, comparisons, boolean operations).

use super::compile_and_run;

// =============================================================================
// Simple if-else tests
// =============================================================================

#[test]
#[ignore]
fn test_if_else_true_branch() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    if true {
        42
    } else {
        0
    }
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
fn test_if_else_false_branch() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    if false {
        0
    } else {
        42
    }
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
fn test_if_else_with_comparison() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 10;
    if x > 5 {
        42
    } else {
        0
    }
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

// =============================================================================
// Comparison operator tests
// =============================================================================

#[test]
#[ignore]
fn test_equal_true() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 5;
    if x == 5 {
        42
    } else {
        0
    }
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
fn test_equal_false() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 5;
    if x == 10 {
        0
    } else {
        42
    }
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
fn test_not_equal_true() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 5;
    if x != 10 {
        42
    } else {
        0
    }
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
fn test_less_than_true() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 3;
    if x < 5 {
        42
    } else {
        0
    }
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
fn test_less_than_false() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 10;
    if x < 5 {
        0
    } else {
        42
    }
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
fn test_greater_than_true() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 10;
    if x > 5 {
        42
    } else {
        0
    }
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
fn test_less_than_or_equal_equal() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 5;
    if x <= 5 {
        42
    } else {
        0
    }
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
fn test_less_than_or_equal_less() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 3;
    if x <= 5 {
        42
    } else {
        0
    }
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
fn test_greater_than_or_equal_equal() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 5;
    if x >= 5 {
        42
    } else {
        0
    }
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
fn test_greater_than_or_equal_greater() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 10;
    if x >= 5 {
        42
    } else {
        0
    }
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

// =============================================================================
// Boolean operation tests
// =============================================================================

#[test]
#[ignore]
fn test_bool_and_true() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    if true and true {
        42
    } else {
        0
    }
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
fn test_bool_and_false() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    if true and false {
        0
    } else {
        42
    }
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
fn test_bool_or_true() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    if lang.i1_or(false, true) {
        42
    } else {
        0
    }
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
fn test_bool_or_false() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    if lang.i1_or(false, false) {
        0
    } else {
        42
    }
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
fn test_bool_not_true() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    if not false {
        42
    } else {
        0
    }
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
fn test_bool_not_false() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    if not true {
        0
    } else {
        42
    }
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

// =============================================================================
// Nested if-else tests
// =============================================================================

#[test]
#[ignore]
fn test_nested_if_else() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 10;
    if x > 5 {
        if x > 15 {
            0
        } else {
            42
        }
    } else {
        0
    }
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
fn test_if_else_chain() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 10;
    if x < 5 {
        0
    } else if x < 15 {
        42
    } else {
        0
    }
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

// =============================================================================
// Complex expressions
// =============================================================================

#[test]
#[ignore]
fn test_comparison_in_expression() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let a = 5;
    let b = 10;
    if lang.i1_and(lang.i64_signed_lt(a, b), lang.i64_signed_gt(b, 5)) {
        42
    } else {
        0
    }
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
fn test_variable_from_branch() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 10;
    let y = if x > 5 { 40 } else { 0 };
    y + 2
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}
