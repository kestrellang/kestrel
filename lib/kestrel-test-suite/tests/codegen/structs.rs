//! Struct construction, field access, nesting, and mutability tests.

use kestrel_test_suite::*;

// =============================================================================
// Basic Construction and Field Access
// =============================================================================

#[test]
fn struct_construction() {
    Test::new(
        r#"module Test

struct Point {
    let x: std.num.Int64
    let y: std.num.Int64
}

func main() -> lang.i64 {
    let p = Point(x: 42, y: 0);
    if p.x != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn struct_field_access() {
    Test::new(
        r#"module Test

struct Point {
    let x: std.num.Int64
    let y: std.num.Int64
}

func main() -> lang.i64 {
    let p = Point(x: 10, y: 32);
    if p.x + p.y != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn struct_multiple_fields() {
    Test::new(
        r#"module Test

struct Data {
    let a: std.num.Int64
    let b: std.num.Int64
    let c: std.num.Int64
}

func main() -> lang.i64 {
    let d = Data(a: 10, b: 20, c: 12);
    if d.a + d.b + d.c != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn struct_second_field() {
    Test::new(
        r#"module Test

struct Point {
    let x: std.num.Int64
    let y: std.num.Int64
}

func main() -> lang.i64 {
    let p = Point(x: 0, y: 42);
    if p.y != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Nesting
// =============================================================================

#[test]
fn nested_struct_extra() {
    Test::new(
        r#"module Test

struct Inner {
    let value: std.num.Int64
}

struct Outer {
    let inner: Inner
    let extra: std.num.Int64
}

func main() -> lang.i64 {
    let o = Outer(inner: Inner(value: 40), extra: 42);
    if o.extra != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn nested_struct_inner_value() {
    Test::new(
        r#"module Test

struct Inner {
    let value: std.num.Int64
}

struct Outer {
    let inner: Inner
    let extra: std.num.Int64
}

func main() -> lang.i64 {
    let o = Outer(inner: Inner(value: 42), extra: 0);
    if o.inner.value != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn nested_struct_combined() {
    Test::new(
        r#"module Test

struct Inner {
    let value: std.num.Int64
}

struct Outer {
    let inner: Inner
    let extra: std.num.Int64
}

func main() -> lang.i64 {
    let o = Outer(inner: Inner(value: 40), extra: 2);
    if o.inner.value + o.extra != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn deeply_nested_struct() {
    Test::new(
        r#"module Test

struct Level3 {
    let value: std.num.Int64
}

struct Level2 {
    let inner: Level3
    let bonus: std.num.Int64
}

struct Level1 {
    let middle: Level2
    let top: std.num.Int64
}

func main() -> lang.i64 {
    let obj = Level1(
        middle: Level2(
            inner: Level3(value: 10),
            bonus: 20
        ),
        top: 12
    );
    // 10 + 20 + 12 = 42
    if obj.middle.inner.value + obj.middle.bonus + obj.top != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn nested_struct_construction_inline() {
    Test::new(
        r#"module Test

struct Point {
    let x: std.num.Int64
    let y: std.num.Int64
}

struct Rectangle {
    let origin: Point
    let size: Point
}

func main() -> lang.i64 {
    let rect = Rectangle(
        origin: Point(x: 5, y: 10),
        size: Point(x: 20, y: 7)
    );
    // 5 + 10 + 20 + 7 = 42
    if rect.origin.x + rect.origin.y + rect.size.x + rect.size.y != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn nested_struct_partial_access() {
    Test::new(
        r#"module Test

struct Inner {
    let a: std.num.Int64
    let b: std.num.Int64
}

struct Outer {
    let inner: Inner
}

func sum_inner(i: Inner) -> std.num.Int64 {
    i.a + i.b
}

func main() -> lang.i64 {
    let o = Outer(inner: Inner(a: 20, b: 22));
    // Access the intermediate inner struct and pass it
    if sum_inner(o.inner) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Returning from Functions
// =============================================================================

#[test]
fn struct_return_from_function() {
    Test::new(
        r#"module Test

struct Point {
    let x: std.num.Int64
    let y: std.num.Int64
}

func make_point(x: std.num.Int64, y: std.num.Int64) -> Point {
    Point(x: x, y: y)
}

func main() -> lang.i64 {
    let p = make_point(10, 32);
    if p.x + p.y != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn struct_return_nested() {
    Test::new(
        r#"module Test

struct Inner {
    let value: std.num.Int64
}

struct Outer {
    let inner: Inner
    let extra: std.num.Int64
}

func make_outer(v: std.num.Int64, e: std.num.Int64) -> Outer {
    Outer(inner: Inner(value: v), extra: e)
}

func main() -> lang.i64 {
    let o = make_outer(40, 2);
    if o.inner.value + o.extra != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn struct_return_modified() {
    Test::new(
        r#"module Test

struct Point {
    var x: std.num.Int64
    var y: std.num.Int64
}

func make_and_modify(base: std.num.Int64) -> Point {
    var p = Point(x: base, y: base);
    p.x = p.x + 10;
    p.y = p.y + 12;
    p
}

func main() -> lang.i64 {
    let p = make_and_modify(10);
    // x = 10 + 10 = 20, y = 10 + 12 = 22, sum = 42
    if p.x + p.y != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Passing to Functions
// =============================================================================

#[test]
fn struct_pass_to_function() {
    Test::new(
        r#"module Test

struct Point {
    let x: std.num.Int64
    let y: std.num.Int64
}

func sum_point(p: Point) -> std.num.Int64 {
    p.x + p.y
}

func main() -> lang.i64 {
    let p = Point(x: 20, y: 22);
    if sum_point(p) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn struct_pass_by_value_copy() {
    Test::new(
        r#"module Test

struct Counter {
    var value: std.num.Int64
}

func increment_copy(c: Counter) -> std.num.Int64 {
    // This is a copy, original is not modified
    c.value + 1
}

func main() -> lang.i64 {
    let c = Counter(value: 41);
    let result = increment_copy(c);
    // Result should be 42, but original c.value is still 41
    if result != 42 { return 1 }
    if c.value != 41 { return 2 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn struct_pass_nested() {
    Test::new(
        r#"module Test

struct Inner {
    let value: std.num.Int64
}

struct Outer {
    let inner: Inner
    let extra: std.num.Int64
}

func sum_outer(o: Outer) -> std.num.Int64 {
    o.inner.value + o.extra
}

func main() -> lang.i64 {
    let o = Outer(inner: Inner(value: 30), extra: 12);
    if sum_outer(o) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn struct_pass_multiple() {
    Test::new(
        r#"module Test

struct Point {
    let x: std.num.Int64
    let y: std.num.Int64
}

func add_points(a: Point, b: Point) -> Point {
    Point(x: a.x + b.x, y: a.y + b.y)
}

func main() -> lang.i64 {
    let p1 = Point(x: 10, y: 5);
    let p2 = Point(x: 12, y: 15);
    let result = add_points(p1, p2);
    // x = 22, y = 20, sum = 42
    if result.x + result.y != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Mutability (Direct Field Mutation)
// =============================================================================

#[test]
fn struct_var_field_mutation() {
    Test::new(
        r#"module Test

struct Counter {
    var value: std.num.Int64
}

func main() -> lang.i64 {
    var c = Counter(value: 0);
    c.value = 42;
    if c.value != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn struct_var_multiple_mutations() {
    Test::new(
        r#"module Test

struct Counter {
    var value: std.num.Int64
}

func main() -> lang.i64 {
    var c = Counter(value: 0);
    c.value = 10;
    c.value = c.value + 20;
    c.value = c.value + 12;
    if c.value != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn struct_nested_field_mutation() {
    Test::new(
        r#"module Test

struct Inner {
    var value: std.num.Int64
}

struct Outer {
    var inner: Inner
    var extra: std.num.Int64
}

func main() -> lang.i64 {
    var o = Outer(inner: Inner(value: 0), extra: 0);
    o.inner.value = 30;
    o.extra = 12;
    if o.inner.value + o.extra != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Mutability via Methods
// =============================================================================

#[test]
fn struct_mutating_method() {
    Test::new(
        r#"module Test

struct Counter {
    var value: std.num.Int64

    mutating func increment(by: std.num.Int64) {
        self.value = self.value + by;
    }
}

func main() -> lang.i64 {
    var c = Counter(value: 0);
    c.increment(42);
    if c.value != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn struct_mutating_method_multiple_calls() {
    Test::new(
        r#"module Test

struct Counter {
    var value: std.num.Int64

    mutating func increment(by: std.num.Int64) {
        self.value = self.value + by;
    }
}

func main() -> lang.i64 {
    var c = Counter(value: 0);
    c.increment(10);
    c.increment(20);
    c.increment(12);
    if c.value != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn struct_mutating_method_nested() {
    Test::new(
        r#"module Test

struct Inner {
    var value: std.num.Int64

    mutating func setValue(to: std.num.Int64) {
        self.value = to;
    }
}

struct Outer {
    var inner: Inner
}

func main() -> lang.i64 {
    var o = Outer(inner: Inner(value: 0));
    // Call mutating method directly on var inner
    var inner = o.inner;
    inner.setValue(42);
    o.inner = inner;
    if o.inner.value != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn struct_mutating_method_returns_value() {
    Test::new(
        r#"module Test

struct Counter {
    var value: std.num.Int64

    mutating func incrementAndGet(by: std.num.Int64) -> std.num.Int64 {
        self.value = self.value + by;
        self.value
    }
}

func main() -> lang.i64 {
    var c = Counter(value: 30);
    let result = c.incrementAndGet(12);
    if result != 42 { return 1 }
    if c.value != 42 { return 2 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Mutability When Passed to Function (mutating parameter)
// =============================================================================

#[test]
fn struct_mutating_parameter() {
    Test::new(
        r#"module Test

struct Counter {
    var value: std.num.Int64
}

func increment(mutating c: Counter, by: std.num.Int64) {
    c.value = c.value + by;
}

func main() -> lang.i64 {
    var c = Counter(value: 0);
    increment(c, 42);
    if c.value != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn struct_mutating_parameter_multiple_calls() {
    Test::new(
        r#"module Test

struct Counter {
    var value: std.num.Int64
}

func increment(mutating c: Counter, by: std.num.Int64) {
    c.value = c.value + by;
}

func main() -> lang.i64 {
    var c = Counter(value: 0);
    increment(c, 10);
    increment(c, 20);
    increment(c, 12);
    if c.value != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn struct_mutating_parameter_nested() {
    Test::new(
        r#"module Test

struct Inner {
    var value: std.num.Int64
}

struct Outer {
    var inner: Inner
}

func set_inner_value(mutating o: Outer, v: std.num.Int64) {
    o.inner.value = v;
}

func main() -> lang.i64 {
    var o = Outer(inner: Inner(value: 0));
    set_inner_value(o, 42);
    if o.inner.value != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn struct_mutating_parameter_method_call() {
    Test::new(
        r#"module Test

struct Counter {
    var value: std.num.Int64

    mutating func increment(by: std.num.Int64) {
        self.value = self.value + by;
    }
}

func double_increment(mutating c: Counter, amount: std.num.Int64) {
    c.increment(amount);
    c.increment(amount);
}

func main() -> lang.i64 {
    var c = Counter(value: 0);
    double_increment(c, 21);
    if c.value != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
