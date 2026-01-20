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

func color_value(c: Color) -> Int {
    match c {
        .Red => 1,
        .Green => 2,
        .Blue => 42
    }
}

func main() -> Int {
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
    case Some(value: Int)
    case None
}

func unwrap_or(opt: Option, default: Int) -> Int {
    match opt {
        .Some(value: v) => v,
        .None => default
    }
}

func main() -> Int {
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
    case Some(value: Int)
    case None
}

func unwrap_or(opt: Option, default: Int) -> Int {
    match opt {
        .Some(value: v) => v,
        .None => default
    }
}

func main() -> Int {
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
    case Ok(value: Int)
    case Err(code: Int)
}

func handle(r: Result) -> Int {
    match r {
        .Ok(value: v) => v,
        .Err(code: c) => c + 100
    }
}

func main() -> Int {
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
    case Ok(value: Int)
    case Err(code: Int)
}

func handle(r: Result) -> Int {
    match r {
        .Ok(value: v) => v,
        .Err(code: c) => c + 100
    }
}

func main() -> Int {
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
    case Some(value: Int)
    case None
}

func add_options(a: Option, b: Option) -> Int {
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

func main() -> Int {
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
