//! Generic function and witness resolution tests.
//!
//! These tests verify that monomorphization and protocol witness resolution
//! work correctly in the codegen backend.

use super::compile_and_run;

// =============================================================================
// Basic Generic Functions
// =============================================================================

#[test]
#[ignore]
fn test_identity_function() {
    let result = compile_and_run(
        r#"
module Test

func identity[T](x: T) -> T {
    x
}

func main() -> Int {
    identity[Int](42)
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

#[test]
#[ignore]
fn test_generic_with_multiple_type_params() {
    let result = compile_and_run(
        r#"
module Test

func first[A, B](a: A, b: B) -> A {
    a
}

func main() -> Int {
    first[Int, Bool](42, true)
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

#[test]
#[ignore]
fn test_generic_second_param() {
    let result = compile_and_run(
        r#"
module Test

func second[A, B](a: A, b: B) -> B {
    b
}

func main() -> Int {
    second[Bool, Int](true, 42)
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

#[test]
#[ignore]
fn test_generic_calling_generic() {
    let result = compile_and_run(
        r#"
module Test

func identity[T](x: T) -> T {
    x
}

func wrap[T](x: T) -> T {
    identity[T](x)
}

func main() -> Int {
    wrap[Int](42)
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

#[test]
#[ignore]
fn test_multiple_instantiations() {
    let result = compile_and_run(
        r#"
module Test

func identity[T](x: T) -> T {
    x
}

func main() -> Int {
    let a = identity[Int](40);
    let b = identity[Bool](true);
    let c = identity[Int](2);
    a + c
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

// =============================================================================
// Generic Structs
// =============================================================================

#[test]
#[ignore]
fn test_generic_struct() {
    let result = compile_and_run(
        r#"
module Test

struct Box[T] {
    let value: T
}

func main() -> Int {
    let b = Box[Int](value: 42);
    b.value
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

#[test]
#[ignore]
fn test_generic_struct_multiple_fields() {
    let result = compile_and_run(
        r#"
module Test

struct Pair[A, B] {
    let first: A
    let second: B
}

func main() -> Int {
    let p = Pair[Int, Int](first: 40, second: 2);
    p.first + p.second
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

#[test]
#[ignore]
fn test_generic_function_with_generic_struct() {
    let result = compile_and_run(
        r#"
module Test

struct Box[T] {
    let value: T
}

func unbox[T](b: Box[T]) -> T {
    b.value
}

func main() -> Int {
    let b = Box[Int](value: 42);
    unbox[Int](b)
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

// =============================================================================
// Protocol Witnesses - Basic
// =============================================================================

#[test]
#[ignore]
fn test_simple_protocol_witness() {
    let result = compile_and_run(
        r#"
module Test

protocol Valuable {
    func value() -> Int
}

struct Token: Valuable {
    func value() -> Int {
        42
    }
}

func get_value[T](x: T) -> Int where T: Valuable {
    x.value()
}

func main() -> Int {
    let t = Token();
    get_value[Token](t)
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

#[test]
#[ignore]
fn test_protocol_witness_with_data() {
    let result = compile_and_run(
        r#"
module Test

protocol Valuable {
    func value() -> Int
}

struct Box: Valuable {
    let inner: Int
    
    func value() -> Int {
        self.inner
    }
}

func get_value[T](x: T) -> Int where T: Valuable {
    x.value()
}

func main() -> Int {
    let b = Box(inner: 42);
    get_value[Box](b)
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

#[test]
#[ignore]
fn test_protocol_multiple_methods() {
    let result = compile_and_run(
        r#"
module Test

protocol Math {
    func add(other: Self) -> Self
    func value() -> Int
}

struct Num: Math {
    let n: Int
    
    func add(other: Num) -> Num {
        Num(n: self.n + other.n)
    }
    
    func value() -> Int {
        self.n
    }
}

func sum_and_get[T](a: T, b: T) -> Int where T: Math {
    let result = a.add(b);
    result.value()
}

func main() -> Int {
    let a = Num(n: 20);
    let b = Num(n: 22);
    sum_and_get[Num](a, b)
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

// =============================================================================
// Generic Witnesses
// =============================================================================

#[test]
#[ignore]
fn test_generic_struct_witness() {
    let result = compile_and_run(
        r#"
module Test

protocol Container {
    func get() -> Int
}

struct Wrapper[T]: Container {
    let value: Int
    
    func get() -> Int {
        self.value
    }
}

func extract[C](c: C) -> Int where C: Container {
    c.get()
}

func main() -> Int {
    let w = Wrapper[Bool](value: 42);
    extract[Wrapper[Bool]](w)
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

#[test]
#[ignore]
fn test_generic_witness_multiple_instantiations() {
    let result = compile_and_run(
        r#"
module Test

protocol Container {
    func get() -> Int
}

struct Box[T]: Container {
    let value: Int
    
    func get() -> Int {
        self.value
    }
}

func extract[C](c: C) -> Int where C: Container {
    c.get()
}

func main() -> Int {
    let b1 = Box[Int](value: 20);
    let b2 = Box[Bool](value: 22);
    extract[Box[Int]](b1) + extract[Box[Bool]](b2)
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

// =============================================================================
// Extension Witnesses
// =============================================================================

#[test]
#[ignore]
fn test_extension_witness() {
    let result = compile_and_run(
        r#"
module Test

protocol Doubler {
    func double() -> Int
}

struct Num {
    let value: Int
}

extend Num: Doubler {
    func double() -> Int {
        self.value * 2
    }
}

func do_double[T](x: T) -> Int where T: Doubler {
    x.double()
}

func main() -> Int {
    let n = Num(value: 21);
    do_double[Num](n)
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

// =============================================================================
// Nested Generic Calls
// =============================================================================

#[test]
#[ignore]
fn test_nested_generic_witness_calls() {
    let result = compile_and_run(
        r#"
module Test

protocol Valuable {
    func value() -> Int
}

struct Token: Valuable {
    let v: Int
    
    func value() -> Int {
        self.v
    }
}

func get_value[T](x: T) -> Int where T: Valuable {
    x.value()
}

func double_value[T](x: T) -> Int where T: Valuable {
    get_value[T](x) * 2
}

func main() -> Int {
    let t = Token(v: 21);
    double_value[Token](t)
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

#[test]
#[ignore]
fn test_generic_chain() {
    let result = compile_and_run(
        r#"
module Test

func step1[T](x: T) -> T { x }
func step2[T](x: T) -> T { step1[T](x) }
func step3[T](x: T) -> T { step2[T](x) }

func main() -> Int {
    step3[Int](42)
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}
