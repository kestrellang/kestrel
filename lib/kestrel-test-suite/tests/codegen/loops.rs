//! Loop tests (loop, while, break, continue).

use super::compile_and_run;

// =============================================================================
// Simple loop with break
// =============================================================================

#[test]
#[ignore]
fn test_simple_loop_with_break() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    var x = 0;
    loop {
        x = x + 1;
        if x == 42 {
            break
        }
    }
    x
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

// =============================================================================
// While loop tests
// =============================================================================

#[test]
#[ignore]
fn test_while_loop() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    var x = 0;
    while x < 42 {
        x = x + 1;
    }
    x
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
fn test_while_loop_condition_false() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    var x = 100;
    while x < 42 {
        x = x + 1;
    }
    x
}
"#,
    );
    // Loop body never executes, x stays 100
    if result.exit_code != 100 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 100);
}

#[test]
#[ignore]
fn test_while_loop_decrement() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    var x = 50;
    while x > 42 {
        x = x - 1;
    }
    x
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

// =============================================================================
// Nested loops
// =============================================================================

#[test]
#[ignore]
fn test_nested_loops() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    var sum = 0;
    var i = 0;
    while i < 6 {
        var j = 0;
        while j < 7 {
            sum = sum + 1;
            j = j + 1;
        }
        i = i + 1;
    }
    sum
}
"#,
    );
    // 6 * 7 = 42
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore]
fn test_nested_loops_with_break() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    var sum = 0;
    var i = 0;
    while i < 10 {
        var j = 0;
        while j < 10 {
            sum = sum + 1;
            if sum == 42 {
                break
            }
            j = j + 1;
        }
        if sum == 42 {
            break
        }
        i = i + 1;
    }
    sum
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

// =============================================================================
// Continue tests
// =============================================================================

#[test]
#[ignore]
fn test_continue() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    var sum = 0;
    var i = 0;
    while i < 10 {
        i = i + 1;
        if i == 5 {
            continue
        }
        sum = sum + i;
    }
    sum
}
"#,
    );
    // 1+2+3+4+6+7+8+9+10 = 55-5 = 50
    if result.exit_code != 50 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 50);
}

#[test]
#[ignore]
fn test_continue_in_loop() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    var sum = 0;
    var i = 0;
    loop {
        i = i + 1;
        if i > 10 {
            break
        }
        if i == 5 {
            continue
        }
        sum = sum + i;
    }
    sum
}
"#,
    );
    // 1+2+3+4+6+7+8+9+10 = 50
    if result.exit_code != 50 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 50);
}

// =============================================================================
// Loop with early return
// =============================================================================

#[test]
#[ignore]
fn test_loop_with_early_return() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    var x = 0;
    loop {
        x = x + 1;
        if x == 42 {
            return x
        }
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
fn test_while_loop_with_early_return() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    var x = 0;
    while x < 100 {
        x = x + 1;
        if x == 42 {
            return x
        }
    }
    0
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

// =============================================================================
// Complex loop scenarios
// =============================================================================

#[test]
#[ignore]
fn test_countdown_loop() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    var countdown = 10;
    var result = 0;
    while countdown > 0 {
        result = result + countdown;
        countdown = countdown - 1;
    }
    result
}
"#,
    );
    // 10+9+8+7+6+5+4+3+2+1 = 55
    if result.exit_code != 55 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 55);
}

#[test]
#[ignore]
fn test_factorial_loop() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    var n = 5;
    var result = 1;
    while n > 1 {
        result = result * n;
        n = n - 1;
    }
    result
}
"#,
    );
    // 5! = 120
    // Note: exit codes are typically 0-255, but macOS allows larger values
    if result.exit_code != 120 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 120);
}

#[test]
#[ignore]
fn test_loop_multiple_breaks() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    var x = 0;
    loop {
        x = x + 1;
        if x == 10 {
            break
        }
        if x == 20 {
            break
        }
    }
    x
}
"#,
    );
    // First break at x == 10
    if result.exit_code != 10 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 10);
}

#[test]
#[ignore]
fn test_loop_zero_iterations() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    var x = 42;
    while false {
        x = 0;
    }
    x
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}
