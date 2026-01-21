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

use kestrel_test_suite::*;

#[test]
fn test_mutating_parameter_struct() {
    // This works because structs are already stack-allocated pointers
    Test::new(
        r#"module Test

struct Point {
    var x: std.num.Int64
    var y: std.num.Int64
}

func reset(mutating p: Point) {
    p.x = 42;
    p.y = 0;
}

func main() -> lang.i64 {
    var pt = Point(x: 0, y: 0);
    reset(pt);
    if pt.x != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_borrow_parameter_struct() {
    // Default parameter mode is borrow (read-only reference)
    Test::new(
        r#"module Test

struct Point {
    let x: std.num.Int64
    let y: std.num.Int64
}

func sum(p: Point) -> std.num.Int64 {
    p.x + p.y
}

func main() -> lang.i64 {
    let pt = Point(x: 20, y: 22);
    if sum(pt) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_method_borrow_self() {
    // Methods take self by borrow by default
    Test::new(
        r#"module Test

struct Point {
    let x: std.num.Int64
    let y: std.num.Int64

    func sum() -> std.num.Int64 {
        self.x + self.y
    }
}

func main() -> lang.i64 {
    let pt = Point(x: 20, y: 22);
    if pt.sum() != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_mutating_method() {
    Test::new(
        r#"module Test

struct Counter {
    var count: std.num.Int64

    mutating func increment() {
        self.count = self.count + 1;
    }

    func read() -> std.num.Int64 {
        self.count
    }
}

func main() -> lang.i64 {
    var c = Counter(count: 40);
    c.increment();
    c.increment();
    if c.read() != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_mutating_through_nested_field() {
    Test::new(
        r#"module Test

struct Inner {
    var value: std.num.Int64
}

struct Outer {
    var inner: Inner
}

func setValue(mutating i: Inner, n: std.num.Int64) {
    i.value = n;
}

func main() -> lang.i64 {
    var o = Outer(inner: Inner(value: 0));
    setValue(o.inner, 42);
    if o.inner.value != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
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

fn test_mutating_parameter_int() {
    // This test fails because `x` is in a register, not on the stack.
    // When increment takes `mutating n`, it gets a reference to a COPY of x.
    Test::new(
        r#"module Test

func increment(mutating n: std.num.Int64) {
    n = n + 1;
}

func main() -> lang.i64 {
    var x: std.num.Int64 = 41;
    increment(x);
    if x != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]

fn test_multiple_mutating_calls() {
    // This test fails because `x` is in a register, not on the stack.
    Test::new(
        r#"module Test

func add(mutating n: std.num.Int64, amount: std.num.Int64) {
    n = n + amount;
}

func main() -> lang.i64 {
    var x: std.num.Int64 = 0;
    add(x, 10);
    add(x, 20);
    add(x, 12);
    if x != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_mutating_and_return() {
    // This test works because it returns the value, not the mutated variable
    Test::new(
        r#"module Test

func incrementAndGet(mutating n: std.num.Int64) -> std.num.Int64 {
    n = n + 1;
    n
}

func main() -> lang.i64 {
    var x: std.num.Int64 = 41;
    let result = incrementAndGet(x);
    if result != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
