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

func main() -> lang.i64 {
    identity[lang.i64](42)
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

func main() -> lang.i64 {
    first[lang.i64, lang.i1](42, true)
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

func main() -> lang.i64 {
    second[lang.i1, lang.i64](true, 42)
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

func main() -> lang.i64 {
    wrap[lang.i64](42)
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

func main() -> lang.i64 {
    let a = identity[lang.i64](40);
    let b = identity[lang.i1](true);
    let c = identity[lang.i64](2);
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

func main() -> lang.i64 {
    let b = Box[lang.i64](value: 42);
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

func main() -> lang.i64 {
    let p = Pair[lang.i64, lang.i64](first: 40, second: 2);
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

func main() -> lang.i64 {
    let b = Box[lang.i64](value: 42);
    unbox[lang.i64](b)
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
    func value() -> lang.i64
}

struct Token: Valuable {
    func value() -> lang.i64 {
        42
    }
}

func get_value[T](x: T) -> lang.i64 where T: Valuable {
    x.value()
}

func main() -> lang.i64 {
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
    func value() -> lang.i64
}

struct Box: Valuable {
    let inner: lang.i64
    
    func value() -> lang.i64 {
        self.inner
    }
}

func get_value[T](x: T) -> lang.i64 where T: Valuable {
    x.value()
}

func main() -> lang.i64 {
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
    func value() -> lang.i64
}

struct Num: Math {
    let n: lang.i64
    
    func add(other: Num) -> Num {
        Num(n: self.n + other.n)
    }
    
    func value() -> lang.i64 {
        self.n
    }
}

func sum_and_get[T](a: T, b: T) -> lang.i64 where T: Math {
    let result = a.add(b);
    result.value()
}

func main() -> lang.i64 {
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
    func read() -> lang.i64
}

struct Wrapper[T]: Container {
    let value: lang.i64
    
    func read() -> lang.i64 {
        self.value
    }
}

func extract[C](c: C) -> lang.i64 where C: Container {
    c.get()
}

func main() -> lang.i64 {
    let w = Wrapper[lang.i1](value: 42);
    extract[Wrapper[lang.i1]](w)
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
    func read() -> lang.i64
}

struct Box[T]: Container {
    let value: lang.i64
    
    func read() -> lang.i64 {
        self.value
    }
}

func extract[C](c: C) -> lang.i64 where C: Container {
    c.get()
}

func main() -> lang.i64 {
    let b1 = Box[lang.i64](value: 20);
    let b2 = Box[lang.i1](value: 22);
    extract[Box[lang.i64]](b1) + extract[Box[lang.i1]](b2)
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
    func double() -> lang.i64
}

struct Num {
    let value: lang.i64
}

extend Num: Doubler {
    func double() -> lang.i64 {
        self.value * 2
    }
}

func do_double[T](x: T) -> lang.i64 where T: Doubler {
    x.double()
}

func main() -> lang.i64 {
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
    func value() -> lang.i64
}

struct Token: Valuable {
    let v: lang.i64
    
    func value() -> lang.i64 {
        self.v
    }
}

func get_value[T](x: T) -> lang.i64 where T: Valuable {
    x.value()
}

func double_value[T](x: T) -> lang.i64 where T: Valuable {
    get_value[T](x) * 2
}

func main() -> lang.i64 {
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

func main() -> lang.i64 {
    step3[lang.i64](42)
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}
