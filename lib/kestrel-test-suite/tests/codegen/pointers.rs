//! Pointer and reference operation tests.
//!
//! These tests exercise the codegen for reference and dereference operations
//! which occur implicitly in Kestrel when:
//! - Calling methods (self is passed by reference)
//! - Using mutating parameters
//! - Passing arguments with borrow/mutating access modes
//!
//! NOTE: Tests involving primitives passed by reference may fail because
//! the current codegen stores primitive locals in registers, not on the stack.
//! Taking a reference to a register-allocated value requires spilling it to
//! the stack, and modifications through the reference aren't reflected in
//! the original variable.

use super::compile_and_run;

#[test]
#[ignore]
fn test_mutating_parameter_struct() {
    // This works because structs are already stack-allocated pointers
    let result = compile_and_run(
        r#"
module Test

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func reset(mutating p: Point) {
    p.x = 42;
    p.y = 0;
}

func main() -> lang.i64 {
    var pt = Point(x: 0, y: 0);
    reset(pt);
    pt.x
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
fn test_borrow_parameter_struct() {
    // Default parameter mode is borrow (read-only reference)
    let result = compile_and_run(
        r#"
module Test

struct Point {
    let x: lang.i64
    let y: lang.i64
}

func sum(p: Point) -> lang.i64 {
    p.x + p.y
}

func main() -> lang.i64 {
    let pt = Point(x: 20, y: 22);
    sum(pt)
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
fn test_method_borrow_self() {
    // Methods take self by borrow by default
    let result = compile_and_run(
        r#"
module Test

struct Point {
    let x: lang.i64
    let y: lang.i64
    
    func sum() -> lang.i64 {
        self.x + self.y
    }
}

func main() -> lang.i64 {
    let pt = Point(x: 20, y: 22);
    pt.sum()
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
fn test_mutating_method() {
    let result = compile_and_run(
        r#"
module Test

struct Counter {
    var count: lang.i64
    
    mutating func increment() {
        self.count = self.count + 1;
    }
    
    func read() -> lang.i64 {
        self.count
    }
}

func main() -> lang.i64 {
    var c = Counter(count: 40);
    c.increment();
    c.increment();
    c.get()
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
fn test_mutating_through_nested_field() {
    let result = compile_and_run(
        r#"
module Test

struct Inner {
    var value: lang.i64
}

struct Outer {
    var inner: Inner
}

func setValue(mutating i: Inner, n: lang.i64) {
    i.value = n;
}

func main() -> lang.i64 {
    var o = Outer(inner: Inner(value: 0));
    setValue(o.inner, 42);
    o.inner.value
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

// The following tests involve primitives passed by mutable reference.
// These currently don't work because primitive locals are allocated in SSA registers,
// not on the stack. When taking a reference, we create a temporary copy on the stack.
// The callee modifies the copy, but the original register is unchanged after the call.
//
// FIX NEEDED: Implement stack allocation for "address-taken" locals. This requires:
// 1. Pre-pass to identify locals that have Rvalue::Ref/RefMut applied
// 2. Allocate those locals on the stack instead of in Cranelift Variables
// 3. Use load/store for all accesses to those locals
//
// For now, mutating parameters work correctly for STRUCT types (which are already
// stack-allocated) but not for primitives (lang.i64, lang.i1, lang.f64, etc.).

#[test]
#[ignore = "Requires stack allocation for address-taken locals"]
fn test_mutating_parameter_int() {
    // This test fails because `x` is in a register, not on the stack.
    // When increment takes `mutating n`, it gets a reference to a COPY of x.
    let result = compile_and_run(
        r#"
module Test

func increment(mutating n: lang.i64) {
    n = n + 1;
}

func main() -> lang.i64 {
    var x = 41;
    increment(x);
    x
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore = "Requires stack allocation for address-taken locals"]
fn test_multiple_mutating_calls() {
    // This test fails because `x` is in a register, not on the stack.
    let result = compile_and_run(
        r#"
module Test

func add(mutating n: lang.i64, amount: lang.i64) {
    n = n + amount;
}

func main() -> lang.i64 {
    var x = 0;
    add(x, 10);
    add(x, 20);
    add(x, 12);
    x
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
fn test_mutating_and_return() {
    // NOTE: This test may fail because primitive locals are register-allocated
    let result = compile_and_run(
        r#"
module Test

func incrementAndGet(mutating n: lang.i64) -> lang.i64 {
    n = n + 1;
    n
}

func main() -> lang.i64 {
    var x = 41;
    incrementAndGet(x)
}
"#,
    );
    if result.exit_code != 42 {
        eprintln!("stderr: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}
