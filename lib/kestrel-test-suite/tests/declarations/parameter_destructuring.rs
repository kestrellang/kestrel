//! Tests for function and closure parameter destructuring patterns.
//!
//! These tests verify that:
//! - Function parameters can use destructuring patterns
//! - Closure parameters can use destructuring patterns
//! - Only irrefutable patterns are allowed (tuple, struct, binding, wildcard)
//! - Refutable patterns (enum, literal, range) cause errors
//! - Access modes interact correctly with pattern bindings
//! - Labels work with destructuring patterns

use kestrel_test_suite::*;

// ============================================================================
// FUNCTION PARAMETER - TUPLE DESTRUCTURING
// ============================================================================

mod function_tuple_destructuring {
    use super::*;

    #[test]
    fn basic_tuple_param() {
        Test::new(
            r#"
module Main

func add((a, b): (lang.i64, lang.i64)) -> lang.i64 {
    lang.i64_add(a, b)
}

func test() -> lang.i64 {
    add((1, 2))
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn triple_tuple_param() {
        Test::new(
            r#"
module Main

func first((x, y, z): (lang.i64, lang.i64, lang.i64)) -> lang.i64 {
    x
}

func test() -> lang.i64 {
    first((1, 2, 3))
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn nested_tuple_param() {
        Test::new(
            r#"
module Main

func nested(((a, b), c): ((lang.i64, lang.i64), lang.i64)) -> lang.i64 {
    lang.i64_add(lang.i64_add(a, b), c)
}

func test() -> lang.i64 {
    nested(((1, 2), 3))
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn multiple_tuple_params() {
        Test::new(
            r#"
module Main

func both((a, b): (lang.i64, lang.i64), (c, d): (lang.i64, lang.i64)) -> lang.i64 {
    lang.i64_add(lang.i64_add(a, b), lang.i64_add(c, d))
}

func test() -> lang.i64 {
    both((1, 2), (3, 4))
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// FUNCTION PARAMETER - STRUCT DESTRUCTURING
// ============================================================================

mod function_struct_destructuring {
    use super::*;

    #[test]
    fn basic_struct_param() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func sum(Point { x, y }: Point) -> lang.i64 {
    lang.i64_add(x, y)
}

func test() -> lang.i64 {
    sum(Point(x: 1, y: 2))
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_param_with_rename() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func sum(Point { x: a, y: b }: Point) -> lang.i64 {
    lang.i64_add(a, b)
}

func test() -> lang.i64 {
    sum(Point(x: 1, y: 2))
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_param_with_rest() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func get_x(Point { x, .. }: Point) -> lang.i64 {
    x
}

func test() -> lang.i64 {
    get_x(Point(x: 42, y: 100))
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// FUNCTION PARAMETER - WILDCARD PATTERNS
// ============================================================================

mod function_wildcard_patterns {
    use super::*;

    #[test]
    fn wildcard_in_tuple() {
        Test::new(
            r#"
module Main

func first((a, _): (lang.i64, lang.i64)) -> lang.i64 {
    a
}

func test() -> lang.i64 {
    first((42, 100))
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn multiple_wildcards() {
        Test::new(
            r#"
module Main

func middle((_, b, _): (lang.i64, lang.i64, lang.i64)) -> lang.i64 {
    b
}

func test() -> lang.i64 {
    middle((1, 42, 3))
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn entire_param_wildcard() {
        Test::new(
            r#"
module Main

func ignore(_: lang.i64) -> lang.i64 {
    42
}

func test() -> lang.i64 {
    ignore(100)
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// FUNCTION PARAMETER - LABELED WITH PATTERNS
// ============================================================================

mod function_labeled_patterns {
    use super::*;

    #[test]
    fn labeled_tuple_param() {
        Test::new(
            r#"
module Main

func add(point (x, y): (lang.i64, lang.i64)) -> lang.i64 {
    lang.i64_add(x, y)
}

func test() -> lang.i64 {
    add(point: (1, 2))
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn multiple_labeled_tuple_params() {
        Test::new(
            r#"
module Main

func distance(from (x1, y1): (lang.i64, lang.i64), to (x2, y2): (lang.i64, lang.i64)) -> lang.i64 {
    let dx = lang.i64_sub(x2, x1);
    let dy = lang.i64_sub(y2, y1);
    lang.i64_add(lang.i64_mul(dx, dx), lang.i64_mul(dy, dy))
}

func test() -> lang.i64 {
    distance(from: (0, 0), to: (3, 4))
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn labeled_struct_param() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func sum(point Point { x, y }: Point) -> lang.i64 {
    lang.i64_add(x, y)
}

func test() -> lang.i64 {
    sum(point: Point(x: 1, y: 2))
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// FUNCTION PARAMETER - ACCESS MODES WITH PATTERNS
// ============================================================================

mod function_access_modes {
    use super::*;

    #[test]
    fn borrow_mode_tuple_is_immutable() {
        Test::new(
            r#"
module Main

func read((a, b): (lang.i64, lang.i64)) -> lang.i64 {
    // a and b are immutable in borrow mode
    lang.i64_add(a, b)
}

func test() -> lang.i64 {
    read((1, 2))
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn borrow_mode_cannot_mutate() {
        Test::new(
            r#"
module Main

func try_mutate((a, b): (lang.i64, lang.i64)) -> lang.i64 {
    a = 10;  // ERROR: a is immutable in borrow mode
    a
}
"#,
        )
        .expect(Fails)
        .expect(HasError("immutable"));
    }

    #[test]
    fn mutating_mode_tuple_is_mutable() {
        Test::new(
            r#"
module Main

func mutate(mutating (a, b): (lang.i64, lang.i64)) {
    a = 10;  // OK: mutating mode makes bindings mutable
    b = 20;
}

func test() -> lang.i64 {
    var tuple = (1, 2);
    mutate(tuple);  // mutating is not a label at call site
    tuple.0
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn consuming_mode_tuple() {
        Test::new(
            r#"
module Main

struct Resource {
    var value: lang.i64
}

func consume(consuming (a, b): (Resource, Resource)) -> lang.i64 {
    // a and b are owned and mutable
    a.value = 100;
    lang.i64_add(a.value, b.value)
}

func test() -> lang.i64 {
    let r1 = Resource(value: 1);
    let r2 = Resource(value: 2);
    consume((r1, r2))
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// FUNCTION PARAMETER - ERROR CASES
// ============================================================================

mod function_error_cases {
    use super::*;

    #[test]
    fn tuple_pattern_on_non_tuple_type() {
        Test::new(
            r#"
module Main

func bad((a, b): lang.i64) -> lang.i64 {
    a
}
"#,
        )
        .expect(Fails);
    }

    #[test]
    fn tuple_arity_mismatch() {
        Test::new(
            r#"
module Main

func bad((a, b, c): (lang.i64, lang.i64)) -> lang.i64 {
    a
}
"#,
        )
        .expect(Fails);
    }

    #[test]
    fn duplicate_binding_in_pattern() {
        Test::new(
            r#"
module Main

func bad((a, a): (lang.i64, lang.i64)) -> lang.i64 {
    a
}
"#,
        )
        .expect(Fails)
        .expect(HasError("duplicate"));
    }

    #[test]
    fn duplicate_binding_nested() {
        Test::new(
            r#"
module Main

func bad((x, (x, y)): (lang.i64, (lang.i64, lang.i64))) -> lang.i64 {
    x
}
"#,
        )
        .expect(Fails)
        .expect(HasError("duplicate"));
    }

    // Note: Refutable pattern tests (enum, literal, range) will be added
    // once the parser rejects them. For now, the focus is on irrefutable patterns.
}

// ============================================================================
// FUNCTION PARAMETER - MIXED STYLES
// ============================================================================

mod function_mixed_styles {
    use super::*;

    #[test]
    fn simple_and_destructured_params() {
        Test::new(
            r#"
module Main

func mixed(x: lang.i64, (a, b): (lang.i64, lang.i64)) -> lang.i64 {
    lang.i64_add(x, lang.i64_add(a, b))
}

func test() -> lang.i64 {
    mixed(10, (1, 2))
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn binding_pattern_still_works() {
        // Ensure simple parameters (binding patterns) still work
        Test::new(
            r#"
module Main

func simple(x: lang.i64, y: lang.i64) -> lang.i64 {
    lang.i64_add(x, y)
}

func test() -> lang.i64 {
    simple(1, 2)
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// METHOD PARAMETER - DESTRUCTURING
// ============================================================================

mod method_destructuring {
    use super::*;

    #[test]
    fn method_with_tuple_param() {
        // Note: Kestrel uses implicit self for instance methods
        Test::new(
            r#"
module Main

struct Vector {
    var x: lang.i64
    var y: lang.i64

    func add((dx, dy): (lang.i64, lang.i64)) -> Vector {
        Vector(x: lang.i64_add(self.x, dx), y: lang.i64_add(self.y, dy))
    }
}

func test() -> lang.i64 {
    let v = Vector(x: 1, y: 2);
    let v2 = v.add((10, 20));
    v2.x
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn method_with_labeled_tuple_param() {
        // Note: Kestrel uses implicit self for instance methods
        Test::new(
            r#"
module Main

struct Vector {
    var x: lang.i64
    var y: lang.i64

    func translate(by (dx, dy): (lang.i64, lang.i64)) -> Vector {
        Vector(x: lang.i64_add(self.x, dx), y: lang.i64_add(self.y, dy))
    }
}

func test() -> lang.i64 {
    let v = Vector(x: 1, y: 2);
    let v2 = v.translate(by: (10, 20));
    v2.x
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// CLOSURE PARAMETER - TUPLE DESTRUCTURING
// ============================================================================

mod closure_tuple_destructuring {
    use super::*;

    #[test]
    fn basic_closure_tuple_param() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    let add = { ((a, b): (lang.i64, lang.i64)) in lang.i64_add(a, b) };
    add((1, 2))
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn nested_closure_tuple_param() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    let nested = { (((a, b), c): ((lang.i64, lang.i64), lang.i64)) in
        lang.i64_add(lang.i64_add(a, b), c)
    };
    nested(((1, 2), 3))
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_with_wildcard() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    let first = { ((a, _): (lang.i64, lang.i64)) in a };
    first((42, 100))
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_multiple_params_some_destructured() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    let f = { (x: lang.i64, (a, b): (lang.i64, lang.i64)) in
        lang.i64_add(x, lang.i64_add(a, b))
    };
    f(10, (1, 2))
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// CLOSURE PARAMETER - STRUCT DESTRUCTURING
// ============================================================================

mod closure_struct_destructuring {
    use super::*;

    #[test]
    fn basic_closure_struct_param() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func test() -> lang.i64 {
    let sum = { (Point { x, y }: Point) in lang.i64_add(x, y) };
    sum(Point(x: 1, y: 2))
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_struct_param_with_rest() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func test() -> lang.i64 {
    let get_x = { (Point { x, .. }: Point) in x };
    get_x(Point(x: 42, y: 100))
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// CLOSURE PARAMETER - IMMUTABILITY
// ============================================================================

mod closure_immutability {
    use super::*;

    #[test]
    fn closure_params_are_immutable() {
        // Closure parameters are always immutable (no access modes)
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    let f = { ((a, b): (lang.i64, lang.i64)) in
        // a = 10;  // Would be error: closure params are immutable
        lang.i64_add(a, b)
    };
    f((1, 2))
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_param_cannot_mutate() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    let f = { ((a, b): (lang.i64, lang.i64)) in
        a = 10;  // ERROR: closure params are immutable
        a
    };
    f((1, 2))
}
"#,
        )
        .expect(Fails)
        .expect(HasError("immutable"));
    }
}

// ============================================================================
// CLOSURE PARAMETER - ERROR CASES
// ============================================================================

mod closure_error_cases {
    use super::*;

    #[test]
    fn closure_duplicate_binding() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    let f = { ((a, a): (lang.i64, lang.i64)) in a };
    f((1, 2))
}
"#,
        )
        .expect(Fails)
        .expect(HasError("duplicate"));
    }

    #[test]
    fn closure_tuple_arity_mismatch() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    let f = { ((a, b, c): (lang.i64, lang.i64)) in a };
    f((1, 2))
}
"#,
        )
        .expect(Fails);
    }
}
