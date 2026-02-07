//! Enum construction, pattern matching, nesting, and mutability tests.

use kestrel_test_suite::*;

// =============================================================================
// Basic Enum Construction and Matching
// =============================================================================

#[test]
fn simple_enum() {
    Test::new(
        r#"module Test

enum Color {
    case Red
    case Green
    case Blue
}

func color_value(c: Color) -> std.num.Int64 {
    match c {
        .Red => 1,
        .Green => 2,
        .Blue => 42
    }
}

func main() -> lang.i64 {
    if color_value(Color.Blue) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn enum_with_payload() {
    Test::new(
        r#"module Test

enum Option {
    case Some(value: std.num.Int64)
    case None
}

func unwrap_or(opt: Option, default: std.num.Int64) -> std.num.Int64 {
    match opt {
        .Some(value: v) => v,
        .None => default
    }
}

func main() -> lang.i64 {
    let some = Option.Some(value: 42);
    if unwrap_or(some, 0) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn enum_none_case() {
    Test::new(
        r#"module Test

enum Option {
    case Some(value: std.num.Int64)
    case None
}

func unwrap_or(opt: Option, default: std.num.Int64) -> std.num.Int64 {
    match opt {
        .Some(value: v) => v,
        .None => default
    }
}

func main() -> lang.i64 {
    let none = Option.None;
    if unwrap_or(none, 42) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn match_optional_type_operator() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let opt: std.num.Int64? = .Some(7);
    let val = match opt {
        .Some(v) => v,
        .None => 0
    };
    if val != 7 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn enum_multiple_payloads() {
    Test::new(
        r#"module Test

enum Result {
    case Ok(value: std.num.Int64)
    case Err(code: std.num.Int64)
}

func handle(r: Result) -> std.num.Int64 {
    match r {
        .Ok(value: v) => v,
        .Err(code: c) => c + 100
    }
}

func main() -> lang.i64 {
    let ok = Result.Ok(value: 42);
    if handle(ok) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn enum_err_case() {
    Test::new(
        r#"module Test

enum Result {
    case Ok(value: std.num.Int64)
    case Err(code: std.num.Int64)
}

func handle(r: Result) -> std.num.Int64 {
    match r {
        .Ok(value: v) => v,
        .Err(code: c) => c + 100
    }
}

func main() -> lang.i64 {
    let err = Result.Err(code: 10);
    // code (10) + 100 = 110
    if handle(err) != 110 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn enum_nested_match() {
    Test::new(
        r#"module Test

enum Option {
    case Some(value: std.num.Int64)
    case None
}

func add_options(a: Option, b: Option) -> std.num.Int64 {
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

func main() -> lang.i64 {
    let a = Option.Some(value: 20);
    let b = Option.Some(value: 22);
    if add_options(a, b) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Nesting (Enum in Struct, Struct in Enum)
// =============================================================================

#[test]
fn enum_nested_in_struct() {
    Test::new(
        r#"module Test

enum Status {
    case Active
    case Inactive
    case Pending(reason: std.num.Int64)
}

struct Task {
    let id: std.num.Int64
    let status: Status
}

func main() -> lang.i64 {
    let task = Task(id: 42, status: Status.Active);
    match task.status {
        .Active => {
            if task.id != 42 { return 1 }
            0
        },
        _ => 2
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn struct_nested_in_enum() {
    Test::new(
        r#"module Test

struct Point {
    let x: std.num.Int64
    let y: std.num.Int64
}

enum Shape {
    case Circle(center: Point, radius: std.num.Int64)
    case Rectangle(origin: Point, width: std.num.Int64, height: std.num.Int64)
}

func get_value(s: Shape) -> std.num.Int64 {
    match s {
        .Circle(center: c, radius: r) => c.x + c.y + r,
        .Rectangle(origin: o, width: w, height: h) => o.x + o.y + w + h
    }
}

func main() -> lang.i64 {
    let circle = Shape.Circle(center: Point(x: 10, y: 12), radius: 20);
    // 10 + 12 + 20 = 42
    if get_value(circle) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn deeply_nested_enum_struct() {
    Test::new(
        r#"module Test

struct Inner {
    let value: std.num.Int64
}

enum Middle {
    case Value(inner: Inner)
    case Empty
}

struct Outer {
    let middle: Middle
    let extra: std.num.Int64
}

func extract(o: Outer) -> std.num.Int64 {
    match o.middle {
        .Value(inner: i) => i.value + o.extra,
        .Empty => o.extra
    }
}

func main() -> lang.i64 {
    let outer = Outer(
        middle: Middle.Value(inner: Inner(value: 30)),
        extra: 12
    );
    if extract(outer) != 42 { return 1 }
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
fn enum_return_simple_case() {
    Test::new(
        r#"module Test

enum Direction {
    case Up
    case Down
    case Left
    case Right
}

func get_direction() -> Direction {
    Direction.Up
}

func main() -> lang.i64 {
    let d = get_direction();
    match d {
        .Up => 0,
        _ => 1
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn enum_return_with_payload() {
    Test::new(
        r#"module Test

enum Option {
    case Some(value: std.num.Int64)
    case None
}

func make_some(v: std.num.Int64) -> Option {
    Option.Some(value: v)
}

func main() -> lang.i64 {
    let opt = make_some(42);
    match opt {
        .Some(value: v) => {
            if v != 42 { return 1 }
            0
        },
        .None => 2
    }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn enum_return_different_cases() {
    Test::new(
        r#"module Test

enum Option {
    case Some(value: std.num.Int64)
    case None
}

func maybe_double(v: std.num.Int64, should_double: std.core.Bool) -> Option {
    if should_double {
        Option.Some(value: v * 2)
    } else {
        Option.None
    }
}

func unwrap_or(opt: Option, default: std.num.Int64) -> std.num.Int64 {
    match opt {
        .Some(value: v) => v,
        .None => default
    }
}

func main() -> lang.i64 {
    let doubled = maybe_double(21, true);
    let none = maybe_double(21, false);

    if unwrap_or(doubled, 0) != 42 { return 1 }
    if unwrap_or(none, 99) != 99 { return 2 }

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
fn enum_pass_to_function() {
    Test::new(
        r#"module Test

enum Color {
    case Red
    case Green
    case Blue
}

func is_blue(c: Color) -> std.core.Bool {
    match c {
        .Blue => true,
        _ => false
    }
}

func main() -> lang.i64 {
    if is_blue(Color.Blue) == false { return 1 }
    if is_blue(Color.Red) { return 2 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn enum_pass_with_payload() {
    Test::new(
        r#"module Test

enum Option {
    case Some(value: std.num.Int64)
    case None
}

func double_if_some(opt: Option) -> std.num.Int64 {
    match opt {
        .Some(value: v) => v * 2,
        .None => 0
    }
}

func main() -> lang.i64 {
    let result = double_if_some(Option.Some(value: 21));
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
fn enum_pass_nested() {
    Test::new(
        r#"module Test

struct Data {
    let value: std.num.Int64
}

enum Container {
    case Full(data: Data, extra: std.num.Int64)
    case Empty
}

func extract_sum(c: Container) -> std.num.Int64 {
    match c {
        .Full(data: d, extra: e) => d.value + e,
        .Empty => 0
    }
}

func main() -> lang.i64 {
    let container = Container.Full(data: Data(value: 30), extra: 12);
    if extract_sum(container) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// =============================================================================
// Mutability (Reassignment)
// =============================================================================

#[test]
fn enum_var_reassign() {
    Test::new(
        r#"module Test

enum State {
    case Off
    case On
}

func get_value(s: State) -> std.num.Int64 {
    match s {
        .Off => 0,
        .On => 42
    }
}

func main() -> lang.i64 {
    var state = State.Off;
    if get_value(state) != 0 { return 1 }

    state = State.On;
    if get_value(state) != 42 { return 2 }

    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn enum_var_reassign_payload() {
    Test::new(
        r#"module Test

enum Option {
    case Some(value: std.num.Int64)
    case None
}

func unwrap_or(opt: Option, default: std.num.Int64) -> std.num.Int64 {
    match opt {
        .Some(value: v) => v,
        .None => default
    }
}

func main() -> lang.i64 {
    var opt = Option.None;
    if unwrap_or(opt, 99) != 99 { return 1 }

    opt = Option.Some(value: 42);
    if unwrap_or(opt, 99) != 42 { return 2 }

    opt = Option.Some(value: 100);
    if unwrap_or(opt, 99) != 100 { return 3 }

    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn enum_nested_struct_mutation() {
    Test::new(
        r#"module Test

struct Data {
    var value: std.num.Int64
}

enum Container {
    case Full(data: Data)
    case Empty
}

func extract(c: Container) -> std.num.Int64 {
    match c {
        .Full(data: d) => d.value,
        .Empty => 0
    }
}

func main() -> lang.i64 {
    var container = Container.Full(data: Data(value: 10));
    if extract(container) != 10 { return 1 }

    // Reassign the entire enum with a new struct
    container = Container.Full(data: Data(value: 42));
    if extract(container) != 42 { return 2 }

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
fn enum_mutating_reassign() {
    Test::new(
        r#"module Test

enum State {
    case Off
    case On
}

func get_value(s: State) -> std.num.Int64 {
    match s {
        .Off => 0,
        .On => 1
    }
}

func toggle(mutating s: State) {
    s = match s {
        .Off => State.On,
        .On => State.Off
    };
}

func main() -> lang.i64 {
    var state = State.Off;
    if get_value(state) != 0 { return 1 }

    toggle(state);
    if get_value(state) != 1 { return 2 }

    toggle(state);
    if get_value(state) != 0 { return 3 }

    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn enum_mutating_payload_access() {
    Test::new(
        r#"module Test

enum Option {
    case Some(value: std.num.Int64)
    case None
}

func double_in_place(mutating opt: Option) {
    opt = match opt {
        .Some(value: v) => Option.Some(value: v * 2),
        .None => Option.None
    };
}

func unwrap_or(opt: Option, default: std.num.Int64) -> std.num.Int64 {
    match opt {
        .Some(value: v) => v,
        .None => default
    }
}

func main() -> lang.i64 {
    var opt = Option.Some(value: 21);
    double_in_place(opt);
    if unwrap_or(opt, 0) != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn enum_mutating_set_to_none() {
    Test::new(
        r#"module Test

enum Option {
    case Some(value: std.num.Int64)
    case None
}

func is_some(opt: Option) -> std.core.Bool {
    match opt {
        .Some(value: _) => true,
        .None => false
    }
}

func clear(mutating opt: Option) {
    opt = Option.None;
}

func main() -> lang.i64 {
    var opt = Option.Some(value: 42);
    if is_some(opt) == false { return 1 }

    clear(opt);
    if is_some(opt) { return 2 }

    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
