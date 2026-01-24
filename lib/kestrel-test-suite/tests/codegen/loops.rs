//! Loop tests (loop, while, break, continue).

use kestrel_test_suite::*;

// =============================================================================
// Simple loop with break
// =============================================================================

#[test]
fn test_simple_loop_with_break() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    var x: std.num.Int64 = 0;
    loop {
        x = x + 1;
        if x == 42 {
            break
        }
    }
    if x != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// While loop tests
// =============================================================================

#[test]
fn test_while_loop() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    var x: std.num.Int64 = 0;
    while x < 42 {
        x = x + 1;
    }
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
fn test_while_loop_condition_false() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    var x: std.num.Int64 = 100;
    while x < 42 {
        x = x + 1;
    }
    // Loop body never executes, x stays 100
    if x != 100 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_while_loop_decrement() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    var x: std.num.Int64 = 50;
    while x > 42 {
        x = x - 1;
    }
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
fn test_while_let_optional_type_operator() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    var opt: std.num.Int64? = .Some(3);
    var sum: std.num.Int64 = 0;
    while let .Some(v) = opt {
        sum = sum + v;
        opt = .None;
    }
    if sum != 3 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Nested loops
// =============================================================================

#[test]
fn test_nested_loops() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    var sum: std.num.Int64 = 0;
    var i: std.num.Int64 = 0;
    while i < 6 {
        var j: std.num.Int64 = 0;
        while j < 7 {
            sum = sum + 1;
            j = j + 1;
        }
        i = i + 1;
    }
    // 6 * 7 = 42
    if sum != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_nested_loops_with_break() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    var sum: std.num.Int64 = 0;
    var i: std.num.Int64 = 0;
    while i < 10 {
        var j: std.num.Int64 = 0;
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
    if sum != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Continue tests
// =============================================================================

#[test]
fn test_continue() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    var sum: std.num.Int64 = 0;
    var i: std.num.Int64 = 0;
    while i < 10 {
        i = i + 1;
        if i == 5 {
            continue
        }
        sum = sum + i;
    }
    // 1+2+3+4+6+7+8+9+10 = 55-5 = 50
    if sum != 50 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_continue_in_loop() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    var sum: std.num.Int64 = 0;
    var i: std.num.Int64 = 0;
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
    // 1+2+3+4+6+7+8+9+10 = 50
    if sum != 50 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Loop with early return
// =============================================================================

#[test]
fn test_loop_with_early_return() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    var x: std.num.Int64 = 0;
    loop {
        x = x + 1;
        if x == 42 {
            return 0
        }
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_while_loop_with_early_return() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    var x: std.num.Int64 = 0;
    while x < 100 {
        x = x + 1;
        if x == 42 {
            return 0
        }
    }
    1
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Complex loop scenarios
// =============================================================================

#[test]
fn test_countdown_loop() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    var countdown: std.num.Int64 = 10;
    var result: std.num.Int64 = 0;
    while countdown > 0 {
        result = result + countdown;
        countdown = countdown - 1;
    }
    // 10+9+8+7+6+5+4+3+2+1 = 55
    if result != 55 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_factorial_loop() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    var n: std.num.Int64 = 5;
    var result: std.num.Int64 = 1;
    while n > 1 {
        result = result * n;
        n = n - 1;
    }
    // 5! = 120
    if result != 120 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_loop_multiple_breaks() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    var x: std.num.Int64 = 0;
    loop {
        x = x + 1;
        if x == 10 {
            break
        }
        if x == 20 {
            break
        }
    }
    // First break at x == 10
    if x != 10 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_loop_zero_iterations() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    var x: std.num.Int64 = 42;
    while false {
        x = 0;
    }
    if x != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
