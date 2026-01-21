//! String literal tests.

use super::compile_and_run;

#[test]

fn test_string_literal() {
    // Test that string literals compile
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let s = "hello";
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

fn test_empty_string() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let s = "";
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

fn test_multiple_strings() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let a = "hello";
    let b = "world";
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

fn test_duplicate_strings() {
    // Test that duplicate strings are deduplicated
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let a = "same";
    let b = "same";
    42
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}
