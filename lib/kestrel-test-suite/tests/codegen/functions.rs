//! Function call tests.

use super::compile_and_run;

#[test]
#[ignore]
fn test_call_simple_function() {
    let result = compile_and_run(
        r#"
module Test

func add(a: lang.i64, b: lang.i64) -> lang.i64 {
    a + b
}

func main() -> lang.i64 {
    add(20, 22)
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
fn test_call_function_no_args() {
    let result = compile_and_run(
        r#"
module Test

func get_answer() -> lang.i64 {
    42
}

func main() -> lang.i64 {
    get_answer()
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
fn test_call_function_chain() {
    let result = compile_and_run(
        r#"
module Test

func double(x: lang.i64) -> lang.i64 {
    x * 2
}

func add_ten(x: lang.i64) -> lang.i64 {
    x + 10
}

func main() -> lang.i64 {
    add_ten(double(16))
}
"#,
    );
    // double(16) = 32, add_ten(32) = 42
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore]
fn test_call_unit_function() {
    let result = compile_and_run(
        r#"
module Test

func do_nothing() {
}

func main() -> lang.i64 {
    do_nothing();
    42
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
fn test_call_multiple_functions() {
    let result = compile_and_run(
        r#"
module Test

func mul(a: lang.i64, b: lang.i64) -> lang.i64 {
    a * b
}

func add(a: lang.i64, b: lang.i64) -> lang.i64 {
    a + b
}

func main() -> lang.i64 {
    add(mul(6, 7), 0)
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
fn test_call_with_local_variables() {
    let result = compile_and_run(
        r#"
module Test

func square(x: lang.i64) -> lang.i64 {
    x * x
}

func main() -> lang.i64 {
    let a = 6;
    let b = square(a);
    b + 6
}
"#,
    );
    // square(6) = 36, 36 + 6 = 42
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore]
fn test_nested_function_calls() {
    let result = compile_and_run(
        r#"
module Test

func add(a: lang.i64, b: lang.i64) -> lang.i64 {
    a + b
}

func main() -> lang.i64 {
    add(add(10, 12), add(10, 10))
}
"#,
    );
    // add(10, 12) = 22, add(10, 10) = 20, add(22, 20) = 42
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore]
fn test_recursive_factorial() {
    let result = compile_and_run(
        r#"
module Test

func factorial(n: lang.i64) -> lang.i64 {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}

func main() -> lang.i64 {
    factorial(5)
}
"#,
    );
    // 5! = 120, but exit codes are limited to 0-255
    // Let's use a smaller test value
    if result.exit_code != 120 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 120);
}
