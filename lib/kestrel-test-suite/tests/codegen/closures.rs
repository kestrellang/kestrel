//! Closure tests: passing, returning, nesting in structs/enums, generics, and captures.

use kestrel_test_suite::*;

// =============================================================================
// Basic Function Pointer Usage
// =============================================================================

// Note: Function pointer tests are ignored because function references (passing named
// functions as values) are not yet fully implemented in codegen.

#[test]

fn function_as_value() {
    Test::new(
        r#"module Test

func add_one(x: std.num.Int64) -> std.num.Int64 {
    x + 1
}

func apply(f: (std.num.Int64) -> std.num.Int64, x: std.num.Int64) -> std.num.Int64 {
    f(x)
}

func main() -> lang.i64 {
    if apply(add_one, 41) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]

fn function_pointer_call() {
    Test::new(
        r#"module Test

func double(x: std.num.Int64) -> std.num.Int64 {
    x * 2
}

func main() -> lang.i64 {
    let f = double;
    if f(21) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]

fn function_pointer_no_args() {
    Test::new(
        r#"module Test

func get_answer() -> std.num.Int64 {
    42
}

func call_it(f: () -> std.num.Int64) -> std.num.Int64 {
    f()
}

func main() -> lang.i64 {
    if call_it(get_answer) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]

fn function_pointer_multiple_args() {
    Test::new(
        r#"module Test

func add(a: std.num.Int64, b: std.num.Int64) -> std.num.Int64 {
    a + b
}

func apply_binary(f: (std.num.Int64, std.num.Int64) -> std.num.Int64, x: std.num.Int64, y: std.num.Int64) -> std.num.Int64 {
    f(x, y)
}

func main() -> lang.i64 {
    if apply_binary(add, 20, 22) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Returning Closures from Functions
// =============================================================================

#[test]
fn closure_return_simple() {
    Test::new(
        r#"module Test

func make_doubler() -> (std.num.Int64) -> std.num.Int64 {
    { (x) in x * 2 }
}

func main() -> lang.i64 {
    let doubler = make_doubler();
    if doubler(21) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn closure_return_with_capture() {
    Test::new(
        r#"module Test

func make_adder(n: std.num.Int64) -> (std.num.Int64) -> std.num.Int64 {
    { (x) in x + n }
}

func main() -> lang.i64 {
    let add10 = make_adder(10);
    if add10(32) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn closure_return_nested() {
    Test::new(
        r#"module Test

func make_curried_add() -> (std.num.Int64) -> (std.num.Int64) -> std.num.Int64 {
    { (a) in { (b) in a + b } }
}

func main() -> lang.i64 {
    let curried = make_curried_add();
    let add20 = curried(20);
    if add20(22) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Passing Closures to Functions
// =============================================================================

#[test]
fn closure_pass_as_argument() {
    Test::new(
        r#"module Test

func apply(f: (std.num.Int64) -> std.num.Int64, x: std.num.Int64) -> std.num.Int64 {
    f(x)
}

func main() -> lang.i64 {
    let result = apply({ (x) in x * 2 }, 21);
    if result != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn closure_pass_with_capture() {
    Test::new(
        r#"module Test

func apply(f: (std.num.Int64) -> std.num.Int64, x: std.num.Int64) -> std.num.Int64 {
    f(x)
}

func main() -> lang.i64 {
    let multiplier: std.num.Int64 = 2;
    let result = apply({ (x) in x * multiplier }, 21);
    if result != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn closure_pass_multiple() {
    Test::new(
        r#"module Test

func combine(
    f: (std.num.Int64) -> std.num.Int64,
    g: (std.num.Int64) -> std.num.Int64,
    x: std.num.Int64
) -> std.num.Int64 {
    g(f(x))
}

func main() -> lang.i64 {
    let result = combine(
        { (x) in x + 10 },
        { (x) in x * 2 },
        11
    );
    // (11 + 10) * 2 = 42
    if result != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Closures in Structs
// =============================================================================

#[test]
fn closure_in_struct_field() {
    Test::new(
        r#"module Test

struct Handler {
    let action: (std.num.Int64) -> std.num.Int64
}

func main() -> lang.i64 {
    let h = Handler(action: { (x) in x * 2 });
    if (h.action)(21) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn closure_in_struct_call() {
    Test::new(
        r#"module Test

struct Calculator {
    let compute: (std.num.Int64, std.num.Int64) -> std.num.Int64
}

func run_calc(calc: Calculator, a: std.num.Int64, b: std.num.Int64) -> std.num.Int64 {
    (calc.compute)(a, b)
}

func main() -> lang.i64 {
    let adder = Calculator(compute: { (x, y) in x + y });
    if run_calc(adder, 20, 22) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn closure_in_nested_struct() {
    Test::new(
        r#"module Test

struct Inner {
    let transform: (std.num.Int64) -> std.num.Int64
}

struct Outer {
    let inner: Inner
    let value: std.num.Int64
}

func main() -> lang.i64 {
    let outer = Outer(
        inner: Inner(transform: { (x) in x + 12 }),
        value: 30
    );
    if (outer.inner.transform)(outer.value) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]

fn function_pointer_in_struct() {
    Test::new(
        r#"module Test

struct Handler {
    let f: (std.num.Int64) -> std.num.Int64
}

func triple(x: std.num.Int64) -> std.num.Int64 {
    x * 3
}

func main() -> lang.i64 {
    let h = Handler(f: triple);
    if (h.f)(14) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Closures in Enums
// =============================================================================

#[test]
fn closure_in_enum_payload() {
    Test::new(
        r#"module Test

enum Action {
    case Transform(f: (std.num.Int64) -> std.num.Int64)
    case NoOp
}

func apply_action(a: Action, x: std.num.Int64) -> std.num.Int64 {
    match a {
        .Transform(f: f) => f(x),
        .NoOp => x
    }
}

func main() -> lang.i64 {
    let action = Action.Transform(f: { (x) in x * 2 });
    if apply_action(action, 21) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn closure_in_enum_match() {
    Test::new(
        r#"module Test

enum MaybeTransform {
    case Just(f: (std.num.Int64) -> std.num.Int64)
    case Nothing
}

func main() -> lang.i64 {
    let mt = MaybeTransform.Just(f: { (x) in x + 32 });
    let result = match mt {
        .Just(f: f) => f(10),
        .Nothing => 0
    };
    if result != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Generic Structs with Closures
// =============================================================================

#[test]
fn closure_generic_struct_simple() {
    Test::new(
        r#"module Test

struct Provider[T] {
    let provide: () -> T
}

func main() -> lang.i64 {
    let p = Provider[std.num.Int64](provide: { 42 });
    if (p.provide)() != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn closure_generic_struct_transform() {
    Test::new(
        r#"module Test

struct Transform[T, U] {
    let transform: (T) -> U
}

func main() -> lang.i64 {
    let t = Transform[std.num.Int64, std.num.Int64](transform: { (x) in x * 2 });
    if (t.transform)(21) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn closure_nested_generic() {
    Test::new(
        r#"module Test

struct Box[T] {
    let value: T
}

struct Container[T] {
    let make: () -> Box[T]
}

func main() -> lang.i64 {
    let c = Container[std.num.Int64](make: { Box[std.num.Int64](value: 42) });
    let box = (c.make)();
    if box.value != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Generic Enums with Closures
// =============================================================================

#[test]
fn closure_generic_enum_simple() {
    Test::new(
        r#"module Test

enum MaybeAction[T] {
    case Action(f: (T) -> T)
    case NoAction
}

func apply[T](m: MaybeAction[T], x: T) -> T {
    match m {
        .Action(f: f) => f(x),
        .NoAction => x
    }
}

func main() -> lang.i64 {
    let action = MaybeAction[std.num.Int64].Action(f: { (x) in x + 20 });
    if apply[std.num.Int64](action, 22) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn closure_generic_enum_option() {
    Test::new(
        r#"module Test

enum OptionalTransform[T] {
    case Some(f: (T) -> T)
    case None
}

func apply_or_default[T](opt: OptionalTransform[T], x: T, default: T) -> T {
    match opt {
        .Some(f: f) => f(x),
        .None => default
    }
}

func main() -> lang.i64 {
    let transform = OptionalTransform[std.num.Int64].Some(f: { (x) in x * 2 });
    let none = OptionalTransform[std.num.Int64].None;

    if apply_or_default[std.num.Int64](transform, 21, 0) != 42 { return 1 }
    if apply_or_default[std.num.Int64](none, 21, 42) != 42 { return 2 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]

fn closure_nested_generic_enum() {
    Test::new(
        r#"module Test

struct Wrapper[T] {
    let value: T
}

enum MaybeProvider[T] {
    case Provider(make: () -> Wrapper[T])
    case Empty
}

func main() -> lang.i64 {
    let provider = MaybeProvider[std.num.Int64].Provider(
        make: { Wrapper[std.num.Int64](value: 42) }
    );

    match provider {
        .Provider(make: m) => {
            let w = m();
            if w.value != 42 { return 1 }
        },
        .Empty => { return 2 }
    }

    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Capture Semantics
// =============================================================================

#[test]
fn closure_capture_single() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let captured: std.num.Int64 = 32;
    let f = { (x: std.num.Int64) in x + captured };
    if f(10) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]

fn closure_capture_multiple() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let a: std.num.Int64 = 10;
    let b: std.num.Int64 = 20;
    let c: std.num.Int64 = 12;
    let f = { a + b + c };
    if f() != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn closure_capture_parameter() {
    Test::new(
        r#"module Test

func make_multiplier(factor: std.num.Int64) -> (std.num.Int64) -> std.num.Int64 {
    { (x) in x * factor }
}

func main() -> lang.i64 {
    let times3 = make_multiplier(3);
    if times3(14) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]

fn closure_capture_nested() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let outer: std.num.Int64 = 20;
    let make_inner = { (x: std.num.Int64) in { x + outer } };
    let inner = make_inner(22);
    if inner() != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn closure_capture_struct_field() {
    Test::new(
        r#"module Test

struct Config {
    let multiplier: std.num.Int64
}

func main() -> lang.i64 {
    let config = Config(multiplier: 2);
    let f = { (x: std.num.Int64) in x * config.multiplier };
    if f(21) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn closure_no_capture_uses_param() {
    Test::new(
        r#"module Test

func apply(f: (std.num.Int64) -> std.num.Int64, x: std.num.Int64) -> std.num.Int64 {
    f(x)
}

func main() -> lang.i64 {
    // This closure doesn't capture anything, just uses its parameter
    let result = apply({ (x) in x + 20 }, 22);
    if result != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn closure_capture_shadowing() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 100;  // This will be captured
    // But the closure parameter shadows it
    let f = { (x: std.num.Int64) in x + 20 };
    // The parameter x (22) is used, not the captured x (100)
    if f(22) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn closure_capture_in_loop_context() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let base: std.num.Int64 = 40;
    let f = { (x: std.num.Int64) in base + x };

    var sum: std.num.Int64 = 0;
    var i: std.num.Int64 = 0;
    while i < 2 {
        sum = sum + f(1);
        i = i + 1
    }
    // (40 + 1) + (40 + 1) = 82, but we want 42
    // Let's use a single call: base=40, x=2 => 42
    if f(2) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Function Returning Function
// =============================================================================

#[test]

fn function_returning_function() {
    Test::new(
        r#"module Test

func add_one(x: std.num.Int64) -> std.num.Int64 {
    x + 1
}

func mul_two(x: std.num.Int64) -> std.num.Int64 {
    x * 2
}

func choose(flag: std.core.Bool) -> (std.num.Int64) -> std.num.Int64 {
    if flag {
        mul_two
    } else {
        add_one
    }
}

func main() -> lang.i64 {
    let f = choose(true);
    if f(21) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Implicit it parameter
// =============================================================================

#[test]

fn closure_implicit_it_param() {
    Test::new(
        r#"module Test

func apply(f: (std.num.Int64) -> std.num.Int64, x: std.num.Int64) -> std.num.Int64 {
    f(x)
}

func main() -> lang.i64 {
    if apply({ it * 2 }, 21) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]

fn closure_implicit_it_with_capture() {
    Test::new(
        r#"module Test

func apply(f: (std.num.Int64) -> std.num.Int64, x: std.num.Int64) -> std.num.Int64 {
    f(x)
}

func main() -> lang.i64 {
    let offset: std.num.Int64 = 20;
    if apply({ it + offset }, 22) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Higher-Order Functions
// =============================================================================

#[test]

fn higher_order_composition() {
    Test::new(
        r#"module Test

func compose(
    f: (std.num.Int64) -> std.num.Int64,
    g: (std.num.Int64) -> std.num.Int64
) -> (std.num.Int64) -> std.num.Int64 {
    { (x) in g(f(x)) }
}

func main() -> lang.i64 {
    let add10 = { (x: std.num.Int64) in x + 10 };
    let double = { (x: std.num.Int64) in x * 2 };
    let composed = compose(add10, double);
    // (11 + 10) * 2 = 42
    if composed(11) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn higher_order_apply_twice() {
    Test::new(
        r#"module Test

func apply_twice(f: (std.num.Int64) -> std.num.Int64, x: std.num.Int64) -> std.num.Int64 {
    f(f(x))
}

func main() -> lang.i64 {
    // apply_twice(add10, 22) = (22 + 10) + 10 = 42
    if apply_twice({ (x) in x + 10 }, 22) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
