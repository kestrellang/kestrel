//! Function call tests.

use super::compile_and_run;

#[test]
#[ignore]
fn test_call_simple_function() {
    let result = compile_and_run(
        r#"
module Test

func add(a: Int, b: Int) -> Int {
    a + b
}

func main() -> Int {
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

func get_answer() -> Int {
    42
}

func main() -> Int {
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

func double(x: Int) -> Int {
    x * 2
}

func add_ten(x: Int) -> Int {
    x + 10
}

func main() -> Int {
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

func main() -> Int {
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

func mul(a: Int, b: Int) -> Int {
    a * b
}

func add(a: Int, b: Int) -> Int {
    a + b
}

func main() -> Int {
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

func square(x: Int) -> Int {
    x * x
}

func main() -> Int {
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

func add(a: Int, b: Int) -> Int {
    a + b
}

func main() -> Int {
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

func factorial(n: Int) -> Int {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}

func main() -> Int {
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
