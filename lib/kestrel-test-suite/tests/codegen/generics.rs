//! Generic function and witness resolution tests.
//!
//! These tests verify that monomorphization and protocol witness resolution
//! work correctly in the codegen backend.

use kestrel_test_suite::*;

// =============================================================================
// Basic Generic Functions
// =============================================================================

#[test]
fn test_identity_function() {
    Test::new(
        r#"module Test

func identity[T](x: T) -> T {
    x
}

func main() -> lang.i64 {
    if identity[std.num.Int64](42) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_generic_with_multiple_type_params() {
    Test::new(
        r#"module Test

func first[A, B](a: A, b: B) -> A {
    a
}

func main() -> lang.i64 {
    if first[std.num.Int64, std.core.Bool](42, true) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_generic_second_param() {
    Test::new(
        r#"module Test

func second[A, B](a: A, b: B) -> B {
    b
}

func main() -> lang.i64 {
    if second[std.core.Bool, std.num.Int64](true, 42) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_generic_calling_generic() {
    Test::new(
        r#"module Test

func identity[T](x: T) -> T {
    x
}

func wrap[T](x: T) -> T {
    identity[T](x)
}

func main() -> lang.i64 {
    if wrap[std.num.Int64](42) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_multiple_instantiations() {
    Test::new(
        r#"module Test

func identity[T](x: T) -> T {
    x
}

func main() -> lang.i64 {
    let a = identity[std.num.Int64](40);
    let b = identity[std.core.Bool](true);
    let c = identity[std.num.Int64](2);
    if a + c != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Generic Structs
// =============================================================================

#[test]
fn test_generic_struct() {
    Test::new(
        r#"module Test

struct Box[T] {
    let value: T
}

func main() -> lang.i64 {
    let b = Box[std.num.Int64](value: 42);
    if b.value != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_generic_struct_multiple_fields() {
    Test::new(
        r#"module Test

struct Pair[A, B] {
    let first: A
    let second: B
}

func main() -> lang.i64 {
    let p = Pair[std.num.Int64, std.num.Int64](first: 40, second: 2);
    if p.first + p.second != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_generic_function_with_generic_struct() {
    Test::new(
        r#"module Test

struct Box[T] {
    let value: T
}

func unbox[T](b: Box[T]) -> T {
    b.value
}

func main() -> lang.i64 {
    let b = Box[std.num.Int64](value: 42);
    if unbox[std.num.Int64](b) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Protocol Witnesses - Basic
// =============================================================================

#[test]
fn test_simple_protocol_witness() {
    Test::new(
        r#"module Test

protocol Valuable {
    func value() -> std.num.Int64
}

struct Token: Valuable {
    func value() -> std.num.Int64 {
        42
    }
}

func get_value[T](x: T) -> std.num.Int64 where T: Valuable {
    x.value()
}

func main() -> lang.i64 {
    let t = Token();
    if get_value[Token](t) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_protocol_witness_with_data() {
    Test::new(
        r#"module Test

protocol Valuable {
    func value() -> std.num.Int64
}

struct Box: Valuable {
    let inner: std.num.Int64

    func value() -> std.num.Int64 {
        self.inner
    }
}

func get_value[T](x: T) -> std.num.Int64 where T: Valuable {
    x.value()
}

func main() -> lang.i64 {
    let b = Box(inner: 42);
    if get_value[Box](b) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_protocol_multiple_methods() {
    Test::new(
        r#"module Test

protocol Math {
    func add(other: Self) -> Self
    func value() -> std.num.Int64
}

struct Num: Math {
    let n: std.num.Int64

    func add(other: Num) -> Num {
        Num(n: self.n + other.n)
    }

    func value() -> std.num.Int64 {
        self.n
    }
}

func sum_and_get[T](a: T, b: T) -> std.num.Int64 where T: Math {
    let result = a.add(b);
    result.value()
}

func main() -> lang.i64 {
    let a = Num(n: 20);
    let b = Num(n: 22);
    if sum_and_get[Num](a, b) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Generic Witnesses
// =============================================================================

#[test]
fn test_generic_struct_witness() {
    Test::new(
        r#"module Test

protocol Container {
    func read() -> std.num.Int64
}

struct Wrapper[T]: Container {
    let value: std.num.Int64

    func read() -> std.num.Int64 {
        self.value
    }
}

func extract[C](c: C) -> std.num.Int64 where C: Container {
    c.read()
}

func main() -> lang.i64 {
    let w = Wrapper[std.core.Bool](value: 42);
    if extract[Wrapper[std.core.Bool]](w) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_generic_witness_multiple_instantiations() {
    Test::new(
        r#"module Test

protocol Container {
    func read() -> std.num.Int64
}

struct Box[T]: Container {
    let value: std.num.Int64

    func read() -> std.num.Int64 {
        self.value
    }
}

func extract[C](c: C) -> std.num.Int64 where C: Container {
    c.read()
}

func main() -> lang.i64 {
    let b1 = Box[std.num.Int64](value: 20);
    let b2 = Box[std.core.Bool](value: 22);
    if extract[Box[std.num.Int64]](b1) + extract[Box[std.core.Bool]](b2) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Extension Witnesses
// =============================================================================

#[test]
fn test_extension_witness() {
    Test::new(
        r#"module Test

protocol Doubler {
    func double() -> std.num.Int64
}

struct Num {
    let value: std.num.Int64
}

extend Num: Doubler {
    func double() -> std.num.Int64 {
        self.value * 2
    }
}

func do_double[T](x: T) -> std.num.Int64 where T: Doubler {
    x.double()
}

func main() -> lang.i64 {
    let n = Num(value: 21);
    if do_double[Num](n) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Nested Generic Calls
// =============================================================================

#[test]
fn test_nested_generic_witness_calls() {
    Test::new(
        r#"module Test

protocol Valuable {
    func value() -> std.num.Int64
}

struct Token: Valuable {
    let v: std.num.Int64

    func value() -> std.num.Int64 {
        self.v
    }
}

func get_value[T](x: T) -> std.num.Int64 where T: Valuable {
    x.value()
}

func double_value[T](x: T) -> std.num.Int64 where T: Valuable {
    get_value[T](x) * 2
}

func main() -> lang.i64 {
    let t = Token(v: 21);
    if double_value[Token](t) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_generic_chain() {
    Test::new(
        r#"module Test

func step1[T](x: T) -> T { x }
func step2[T](x: T) -> T { step1[T](x) }
func step3[T](x: T) -> T { step2[T](x) }

func main() -> lang.i64 {
    if step3[std.num.Int64](42) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Generic Type Parameter Property/Method Access via Witness Tables
// =============================================================================

#[test]
fn test_static_function_via_type_parameter() {
    Test::new(
        r#"module Test

protocol Factory {
    static func create() -> Self
}

struct Widget: Factory {
    let value: std.num.Int64
    static func create() -> Self {
        Widget(value: 42)
    }
}

func make[T]() -> T where T: Factory {
    T.create()
}

func main() -> std.num.Int64 {
    let w: Widget = make[Widget]();
    if w.value != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_static_property_via_type_parameter() {
    Test::new(
        r#"module Test

protocol HasDefault {
    static var defaultValue: std.num.Int64 { get }
}

struct Config: HasDefault {
    static var defaultValue: std.num.Int64 { 100 }
}

func getDefault[T]() -> std.num.Int64 where T: HasDefault {
    T.defaultValue
}

func main() -> std.num.Int64 {
    if getDefault[Config]() != 100 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_instance_property_via_type_parameter() {
    Test::new(
        r#"module Test

protocol HasValue {
    var value: std.num.Int64 { get }
}

struct Box: HasValue {
    var value: std.num.Int64 { get { 42 } }
}

func getValue[T](item: T) -> std.num.Int64 where T: HasValue {
    item.value
}

func main() -> std.num.Int64 {
    let b = Box();
    if getValue[Box](b) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_static_mutable_property_via_type_parameter() {
    Test::new(
        r#"module Test

protocol Counter {
    static var count: std.num.Int64 { get set }
}

struct MyCounter: Counter {
    static var _count: std.num.Int64 = 0
    static var count: std.num.Int64 {
        get { MyCounter._count }
        set { MyCounter._count = newValue }
    }
}

func increment[T]() where T: Counter {
    T.count = T.count + 1
}

func main() -> std.num.Int64 {
    increment[MyCounter]();
    if MyCounter.count != 1 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
