//! Function pointer and closure tests.

use super::compile_and_run;

#[test]
#[ignore]
fn test_function_as_value() {
    let result = compile_and_run(
        r#"
module Test

func add_one(x: lang.i64) -> lang.i64 {
    x + 1
}

func apply(f: (lang.i64) -> lang.i64, x: lang.i64) -> lang.i64 {
    f(x)
}

func main() -> lang.i64 {
    apply(add_one, 41)
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
fn test_function_pointer_call() {
    // Test just returning the function address as an lang.i64 (to verify func_addr works)
    let result = compile_and_run(
        r#"
module Test

func double(x: lang.i64) -> lang.i64 {
    x * 2
}

func main() -> lang.i64 {
    double(21)
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("Direct call failed. stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42, "Direct call should work");

    // Now test with function pointer
    let result2 = compile_and_run(
        r#"
module Test

func double(x: lang.i64) -> lang.i64 {
    x * 2
}

func main() -> lang.i64 {
    let f = double;
    f(21)
}
"#,
    );
    if result2.exit_code != 42 {
        eprintln!("Indirect call failed. stderr: {}", result2.stderr);
    }
    assert_eq!(result2.exit_code, 42, "Indirect call should work");
}

#[test]
#[ignore]
fn test_function_pointer_in_struct() {
    let result = compile_and_run(
        r#"
module Test

struct Handler {
    var func: (lang.i64) -> lang.i64;
}

func triple(x: lang.i64) -> lang.i64 {
    x * 3
}

func main() -> lang.i64 {
    let h = Handler (func: triple);
    (h.func)(14)
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
fn test_function_returning_function() {
    let result = compile_and_run(
        r#"
module Test

func add_one(x: lang.i64) -> lang.i64 {
    x + 1
}

func mul_two(x: lang.i64) -> lang.i64 {
    x * 2
}

func choose(flag: lang.i1) -> (lang.i64) -> lang.i64 {
    if flag {
        mul_two
    } else {
        add_one
    }
}

func main() -> lang.i64 {
    let f = choose(true);
    f(21)
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
fn test_function_pointer_no_args() {
    let result = compile_and_run(
        r#"
module Test

func get_answer() -> lang.i64 {
    42
}

func call_it(f: () -> lang.i64) -> lang.i64 {
    f()
}

func main() -> lang.i64 {
    call_it(get_answer)
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
fn test_function_pointer_multiple_args() {
    let result = compile_and_run(
        r#"
module Test

func add(a: lang.i64, b: lang.i64) -> lang.i64 {
    a + b
}

func apply_binary(f: (lang.i64, lang.i64) -> lang.i64, x: lang.i64, y: lang.i64) -> lang.i64 {
    f(x, y)
}

func main() -> lang.i64 {
    apply_binary(add, 20, 22)
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}
