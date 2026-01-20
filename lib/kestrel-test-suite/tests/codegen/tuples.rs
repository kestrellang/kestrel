//! Tuple codegen tests.

use crate::codegen::compile_and_run;

#[test]
#[ignore]
fn test_tuple_construction() {
    let result = compile_and_run(
        r#"
module Test

func main() -> Int {
    let t = (42, 0);
    t.0
}
"#,
    );
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore]
fn test_tuple_second_element() {
    let result = compile_and_run(
        r#"
module Test

func main() -> Int {
    let t = (0, 42);
    t.1
}
"#,
    );
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore]
fn test_tuple_multiple_elements() {
    let result = compile_and_run(
        r#"
module Test

func main() -> Int {
    let t = (10, 20, 12);
    t.0 + t.1 + t.2
}
"#,
    );
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore]
fn test_tuple_mixed_types() {
    let result = compile_and_run(
        r#"
module Test

func main() -> Int {
    let t = (true, 42);
    t.1
}
"#,
    );
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore]
fn test_nested_tuple() {
    // Note: t.0.0 parses as t.(0.0) which is a float literal
    // So we use a temporary variable to work around this parser limitation
    let result = compile_and_run(
        r#"
module Test

func main() -> Int {
    let t = ((40, 2), 0);
    let inner = t.0;
    inner.0 + inner.1
}
"#,
    );
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore]
fn test_tuple_from_function() {
    let result = compile_and_run(
        r#"
module Test

func make_pair(a: Int, b: Int) -> (Int, Int) {
    (a, b)
}

func main() -> Int {
    let t = make_pair(20, 22);
    t.0 + t.1
}
"#,
    );
    assert_eq!(result.exit_code, 42);
}
