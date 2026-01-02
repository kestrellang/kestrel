//! Function pointer and closure tests.

use super::compile_and_run;

#[test]
#[ignore]
fn test_function_as_value() {
    let result = compile_and_run(
        r#"
module Test

func add_one(x: Int) -> Int {
    x + 1
}

func apply(f: (Int) -> Int, x: Int) -> Int {
    f(x)
}

func main() -> Int {
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
    // Test just returning the function address as an int (to verify func_addr works)
    let result = compile_and_run(
        r#"
module Test

func double(x: Int) -> Int {
    x * 2
}

func main() -> Int {
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

func double(x: Int) -> Int {
    x * 2
}

func main() -> Int {
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
    func: (Int) -> Int,
}

func triple(x: Int) -> Int {
    x * 3
}

func main() -> Int {
    let h = Handler { func: triple };
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

func add_one(x: Int) -> Int {
    x + 1
}

func mul_two(x: Int) -> Int {
    x * 2
}

func choose(flag: Bool) -> (Int) -> Int {
    if flag {
        mul_two
    } else {
        add_one
    }
}

func main() -> Int {
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

func get_answer() -> Int {
    42
}

func call_it(f: () -> Int) -> Int {
    f()
}

func main() -> Int {
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

func add(a: Int, b: Int) -> Int {
    a + b
}

func apply_binary(f: (Int, Int) -> Int, x: Int, y: Int) -> Int {
    f(x, y)
}

func main() -> Int {
    apply_binary(add, 20, 22)
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}
