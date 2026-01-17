//! Tests for match expressions.
//!
//! These tests verify that:
//! - Match expressions parse correctly
//! - Match arms are evaluated in order
//! - Match expressions return values
//! - All arms must have compatible types
//! - Guards work correctly

use kestrel_test_suite::*;

// ============================================================================
// BASIC MATCH SYNTAX
// ============================================================================

mod basic_syntax {
    use super::*;

    #[test]
    fn match_on_bool() {
        Test::new(
            r#"
module Main

func test(b: Bool) -> Int {
    match b {
        true => 1,
        false => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn match_on_enum_simple_cases() {
        Test::new(
            r#"
module Main

enum Color {
    case Red
    case Green
    case Blue
}

func test(c: Color) -> Int {
    match c {
        .Red => 1,
        .Green => 2,
        .Blue => 3
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn match_with_block_bodies() {
        Test::new(
            r#"
module Main

enum Color {
    case Red
    case Green
    case Blue
}

func test(c: Color) -> Int {
    match c {
        .Red => {
            let x = 1;
            x
        },
        .Green => {
            let y = 2;
            y
        },
        .Blue => 3
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn match_trailing_comma_optional() {
        Test::new(
            r#"
module Main

func test(b: Bool) -> Int {
    match b {
        true => 1,
        false => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn match_with_trailing_comma() {
        Test::new(
            r#"
module Main

func test(b: Bool) -> Int {
    match b {
        true => 1,
        false => 0,
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn match_as_expression_in_let() {
        Test::new(
            r#"
module Main

func test(b: Bool) -> Int {
    let result = match b {
        true => 42,
        false => 0
    };
    result
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn match_as_return_expression() {
        Test::new(
            r#"
module Main

func test(b: Bool) -> Int {
    return match b {
        true => 1,
        false => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn nested_match_expressions() {
        Test::new(
            r#"
module Main

func test(a: Bool, b: Bool) -> Int {
    match a {
        true => match b {
            true => 1,
            false => 2
        },
        false => 3
    }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// ENUM PATTERNS WITH ASSOCIATED VALUES
// ============================================================================

mod enum_associated_values {
    use super::*;

    #[test]
    fn match_enum_with_associated_value() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Int {
    match opt {
        .Some(value) => value,
        .None => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn match_enum_explicit_label() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Int {
    match opt {
        .Some(value: v) => v,
        .None => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn match_enum_multiple_associated_values() {
        Test::new(
            r#"
module Main

enum Result[T, E] {
    case Ok(value: T)
    case Err(error: E)
}

func test(r: Result[Int, String]) -> Int {
    match r {
        .Ok(value) => value,
        .Err(error) => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn match_enum_ignore_associated_value() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Bool {
    match opt {
        .Some(_) => true,
        .None => false
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn match_nested_enum_patterns() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Option[Int]]) -> Int {
    match opt {
        .Some(value: .Some(inner)) => inner,
        .Some(value: .None) => 0,
        .None => 0
    }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// OR-PATTERNS
// ============================================================================

mod or_patterns {
    use super::*;

    #[test]
    fn or_pattern_simple() {
        Test::new(
            r#"
module Main

enum Color {
    case Red
    case Green
    case Blue
}

func test(c: Color) -> String {
    match c {
        .Red or .Green => "warm-ish",
        .Blue => "cool"
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn or_pattern_multiple() {
        Test::new(
            r#"
module Main

enum Color {
    case Red
    case Orange
    case Yellow
    case Green
    case Blue
    case Purple
}

func test(c: Color) -> String {
    match c {
        .Red or .Orange or .Yellow => "warm",
        .Green or .Blue or .Purple => "cool"
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn or_pattern_with_consistent_bindings() {
        Test::new(
            r#"
module Main

enum Expr {
    case Add(left: Int, right: Int)
    case Sub(left: Int, right: Int)
    case Mul(left: Int, right: Int)
}

func test(e: Expr) -> Int {
    match e {
        .Add(left, right) or .Sub(left, right) => left + right,
        .Mul(left, right) => left * right
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn or_pattern_inconsistent_bindings_error() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Int {
    match opt {
        .Some(value) or .None => value,
        _ => 0
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("inconsistent"));
    }

    #[test]
    fn or_pattern_nested() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> String {
    match opt {
        .Some(value: 1 or 2 or 3) => "small",
        .Some(_) => "large",
        .None => "nothing"
    }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// GUARDS
// ============================================================================

mod guards {
    use super::*;

    #[test]
    fn guard_simple() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> String {
    match opt {
        .Some(n) if n > 0 => "positive",
        .Some(n) if n < 0 => "negative",
        .Some(_) => "zero",
        .None => "nothing"
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn guard_with_binding_in_condition() {
        Test::new(
            r#"
module Main

func test(x: Int) -> String {
    match x {
        n if n > 100 => "big",
        n if n > 10 => "medium",
        _ => "small"
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn guard_must_be_bool() {
        Test::new(
            r#"
module Main

func test(x: Int) -> Int {
    match x {
        n if n => n,
        _ => 0
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("Bool"));
    }

    #[test]
    fn guard_with_or_pattern() {
        Test::new(
            r#"
module Main

enum Value {
    case A(n: Int)
    case B(n: Int)
}

func test(v: Value) -> String {
    match v {
        .A(n) or .B(n) if n > 0 => "positive",
        _ => "other"
    }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// TYPE INFERENCE
// ============================================================================

mod type_inference {
    use super::*;

    #[test]
    fn infer_match_result_type() {
        Test::new(
            r#"
module Main

func test(b: Bool) -> Int {
    let x = match b {
        true => 1,
        false => 0
    };
    x
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn match_arms_must_have_same_type() {
        Test::new(
            r#"
module Main

func test(b: Bool) -> Int {
    match b {
        true => 1,
        false => "zero"
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("type"));
    }

    #[test]
    fn match_on_generic_enum() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func unwrapOr[T](opt: Option[T], default: T) -> T {
    match opt {
        .Some(value) => value,
        .None => default
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_pattern_binding_type() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Int {
    match opt {
        .Some(x) => x + 1,
        .None => 0
    }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// NEVER TYPE PROPAGATION
// ============================================================================

mod never_type {
    use super::*;

    #[test]
    fn match_arm_with_return() {
        Test::new(
            r#"
module Main

func test(b: Bool) -> Int {
    match b {
        true => return 42,
        false => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn match_all_arms_return() {
        Test::new(
            r#"
module Main

func test(b: Bool) -> Int {
    match b {
        true => return 1,
        false => return 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn match_arm_with_break_in_loop() {
        Test::new(
            r#"
module Main

func test() -> Int {
    var result = 0;
    loop {
        match true {
            true => break,
            false => { result = 1; }
        }
    }
    result
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// ERROR CASES
// ============================================================================

mod errors {
    use super::*;

    #[test]
    fn unknown_enum_case() {
        Test::new(
            r#"
module Main

enum Color {
    case Red
    case Green
}

func test(c: Color) -> Int {
    match c {
        .Red => 1,
        .Blue => 2
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("Blue"));
    }

    #[test]
    fn pattern_type_mismatch() {
        Test::new(
            r#"
module Main

func test(x: Int) -> Int {
    match x {
        "hello" => 1,
        _ => 0
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("type"));
    }

    #[test]
    fn duplicate_binding_name() {
        Test::new(
            r#"
module Main

func test(t: (Int, Int)) -> Int {
    match t {
        (x, x) => x
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("duplicate"));
    }

    #[test]
    fn wrong_tuple_arity() {
        Test::new(
            r#"
module Main

func test(t: (Int, Int)) -> Int {
    match t {
        (a, b, c) => a + b + c
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("arity"));
    }

    #[test]
    fn wrong_enum_arity() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Int {
    match opt {
        .Some(a, b) => a + b,
        .None => 0
    }
}
"#,
        )
        .expect(Fails);
    }

    #[test]
    fn float_literal_in_pattern() {
        Test::new(
            r#"
module Main

func test(x: Float) -> Int {
    match x {
        3.14 => 1,
        _ => 0
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("float"));
    }
}

// ============================================================================
// SCOPING
// ============================================================================

mod scoping {
    use super::*;

    #[test]
    fn pattern_binding_scope_limited_to_arm() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Int {
    match opt {
        .Some(x) => x,
        .None => x
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("undefined"));
    }

    #[test]
    fn shadowing_in_match_arm() {
        Test::new(
            r#"
module Main

func test(x: Int) -> Int {
    let y = 100;
    match x {
        y => y + 1
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn outer_variable_accessible_in_match() {
        Test::new(
            r#"
module Main

func test(b: Bool) -> Int {
    let multiplier = 10;
    match b {
        true => multiplier * 2,
        false => multiplier
    }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// REGRESSION TESTS
// ============================================================================

mod regression {
    use super::*;

    /// Test that integer literals in match patterns inherit the scrutinee type (primitive types).
    ///
    /// Previously, integer literal patterns always defaulted to I64, causing
    /// type mismatches when matching against other primitive integer types like lang.i32.
    ///
    /// Issue: Integer literal type inference in match (for primitive types)
    /// Fix: Use expected_ty from scrutinee when resolving integer literal patterns
    #[test]
    fn integer_literal_pattern_inherits_primitive_type() {
        // Note: This tests primitive types (lang.i32), not wrapper structs (Int32)
        // For wrapper structs, ExpressibleByIntLiteral protocol would be needed
        Test::new(
            r#"
module Main

func classify(code: lang.i32) -> lang.i64 {
    match code {
        0 => 1,
        1 => 2,
        2 => 3,
        _ => 0
    }
}
"#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    /// Test with lang.i8 to ensure the fix works for all primitive integer types.
    #[test]
    fn integer_literal_pattern_with_primitive_i8() {
        Test::new(
            r#"
module Main

func test(x: lang.i8) -> lang.i64 {
    match x {
        0 => 1,
        1 => 2,
        _ => 3
    }
}
"#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    /// Test with lang.i16 to ensure the fix works for all primitive integer types.
    #[test]
    fn integer_literal_pattern_with_primitive_i16() {
        Test::new(
            r#"
module Main

func test(x: lang.i16) -> lang.i64 {
    match x {
        42 => 1,
        _ => 0
    }
}
"#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    /// Test that integer literals in match patterns work with wrapper struct types
    /// that conform to ExpressibleByIntLiteral.
    ///
    /// This tests the full type inference path where the literal pattern's type
    /// is inferred from the scrutinee type via the ExpressibleByIntLiteral protocol.
    #[test]
    fn integer_literal_pattern_with_wrapper_struct() {
        Test::new(
            r#"
module Main

@builtin(.ExpressibleByIntLiteral)
protocol ExpressibleByIntLiteral {
    init(intLiteral value: lang.i64)
}

struct MyInt: ExpressibleByIntLiteral {
    var value: lang.i64

    init(intLiteral value: lang.i64) {
        self.value = value
    }
}

func test_match(x: MyInt) -> lang.i64 {
    match x {
        0 => 100,
        1 => 200,
        _ => 300,
    }
}
"#,
        )
        .without_prelude()
        .expect(Compiles);
    }
}
