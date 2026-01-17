//! Arithmetic operation tests.

use super::compile_and_run;

#[test]
#[ignore] // Enable once codegen is working
fn test_return_constant() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
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
fn test_add() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 10;
    let y = 32;
    x + y
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
fn test_subtract() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 50;
    let y = 8;
    x - y
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
fn test_multiply() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 6;
    let y = 7;
    x * y
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
fn test_divide() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 84;
    let y = 2;
    x / y
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
fn test_modulo() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = 142;
    let y = 100;
    x % y
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
fn test_negation() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x = -42;
    -x
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
fn test_complex_expression() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let a = 10;
    let b = 3;
    let c = 2;
    (a + b) * c + (a - b)
}
"#,
    );
    // (10 + 3) * 2 + (10 - 3) = 13 * 2 + 7 = 26 + 7 = 33
    if result.exit_code != 33 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 33);
}

#[test]
#[ignore]
fn test_return_zero() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    0
}
"#,
    );
    if result.exit_code != 0 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 0);
}

#[test]
#[ignore]
fn test_unit_main() {
    let result = compile_and_run(
        r#"
module Test

func main() {
    let x = 42;
}
"#,
    );
    // Unit main returns 0
    if result.exit_code != 0 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 0);
}
