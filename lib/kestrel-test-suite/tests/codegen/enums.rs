//! Enum construction and pattern matching tests.

use super::compile_and_run;

#[test]
#[ignore]
fn test_simple_enum() {
    let result = compile_and_run(
        r#"
module Test

enum Color {
    case Red
    case Green
    case Blue
}

func color_value(c: Color) -> lang.i64 {
    match c {
        .Red => 1,
        .Green => 2,
        .Blue => 42
    }
}

func main() -> lang.i64 {
    color_value(Color.Blue)
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
fn test_enum_with_payload() {
    let result = compile_and_run(
        r#"
module Test

enum Option {
    case Some(value: lang.i64)
    case None
}

func unwrap_or(opt: Option, default: lang.i64) -> lang.i64 {
    match opt {
        .Some(value: v) => v,
        .None => default
    }
}

func main() -> lang.i64 {
    let some = Option.Some(value: 42);
    unwrap_or(some, 0)
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
fn test_enum_none_case() {
    let result = compile_and_run(
        r#"
module Test

enum Option {
    case Some(value: lang.i64)
    case None
}

func unwrap_or(opt: Option, default: lang.i64) -> lang.i64 {
    match opt {
        .Some(value: v) => v,
        .None => default
    }
}

func main() -> lang.i64 {
    let none = Option.None;
    unwrap_or(none, 42)
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
fn test_enum_multiple_payloads() {
    let result = compile_and_run(
        r#"
module Test

enum Result {
    case Ok(value: lang.i64)
    case Err(code: lang.i64)
}

func handle(r: Result) -> lang.i64 {
    match r {
        .Ok(value: v) => v,
        .Err(code: c) => c + 100
    }
}

func main() -> lang.i64 {
    let ok = Result.Ok(value: 42);
    handle(ok)
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
fn test_enum_err_case() {
    let result = compile_and_run(
        r#"
module Test

enum Result {
    case Ok(value: lang.i64)
    case Err(code: lang.i64)
}

func handle(r: Result) -> lang.i64 {
    match r {
        .Ok(value: v) => v,
        .Err(code: c) => c + 100
    }
}

func main() -> lang.i64 {
    let err = Result.Err(code: 10);
    handle(err)
}
"#,
    );
    // code (10) + 100 = 110
    if result.exit_code != 110 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 110);
}

#[test]
#[ignore]
fn test_enum_nested_match() {
    let result = compile_and_run(
        r#"
module Test

enum Option {
    case Some(value: lang.i64)
    case None
}

func add_options(a: Option, b: Option) -> lang.i64 {
    match a {
        .Some(value: x) => {
            match b {
                .Some(value: y) => x + y,
                .None => x
            }
        },
        .None => {
            match b {
                .Some(value: y) => y,
                .None => 0
            }
        }
    }
}

func main() -> lang.i64 {
    let a = Option.Some(value: 20);
    let b = Option.Some(value: 22);
    add_options(a, b)
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}
