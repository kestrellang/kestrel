//! Tests for closure expressions.
//!
//! These tests verify that closures:
//! - Parse correctly with various parameter forms
//! - Support implicit `it` parameter
//! - Capture variables from enclosing scope
//! - Have correct function types
//! - Work with trailing closure syntax
//! - Integrate with type inference

use kestrel_test_suite::*;

// ============================================================================
// BASIC CLOSURE SYNTAX
// ============================================================================

mod basic_syntax {
    use super::*;

    #[test]
    fn closure_no_params_no_in() {
        // Simplest closure: no params, no `in` keyword
        Test::new(
            r#"
module Main

func test() -> () -> Int {
    { 42 }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_empty_params_with_in() {
        // Explicit empty params with `in`
        Test::new(
            r#"
module Main

func test() -> () -> Int {
    { () in 42 }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_single_param_with_type() {
        // Single parameter with explicit type
        Test::new(
            r#"
module Main

func test() -> (Int) -> Int {
    { (x: Int) in x }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_single_param_without_type() {
        // Single parameter, type inferred from context
        Test::new(
            r#"
module Main

func test() -> (Int) -> Int {
    { (x) in x }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_multiple_params_with_types() {
        // Multiple parameters with explicit types
        Test::new(
            r#"
module Main

func test() -> (Int, Int) -> Int {
    { (x: Int, y: Int) in x + y }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_multiple_params_without_types() {
        // Multiple parameters, types inferred
        Test::new(
            r#"
module Main

func test() -> (Int, Int) -> Int {
    { (x, y) in x + y }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_mixed_typed_untyped_params() {
        // Mix of typed and untyped parameters
        Test::new(
            r#"
module Main

func test() -> (Int, String) -> Int {
    { (x: Int, y) in x }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_three_params() {
        // Three parameters
        Test::new(
            r#"
module Main

func test() -> (Int, Int, Int) -> Int {
    { (a, b, c) in a + b + c }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// IMPLICIT `it` PARAMETER
// ============================================================================

mod implicit_it {
    use super::*;

    #[test]
    fn it_with_single_param_context() {
        // `it` used when expected type has 1 parameter
        Test::new(
            r#"
module Main

func test() -> (Int) -> Int {
    { it * 2 }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn it_not_used_zero_param_context() {
        // `it` available but not used, arity 0 is fine
        Test::new(
            r#"
module Main

func test() -> () -> Int {
    { 42 }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn it_not_used_multi_param_context() {
        // `it` available but not used, arity 2+ is fine
        Test::new(
            r#"
module Main

func test() -> (Int, Int) -> Int {
    { 42 }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn it_used_zero_param_context_error() {
        // Error: `it` used but arity is 0
        Test::new(
            r#"
module Main

func test() -> () -> Int {
    { it }
}
"#,
        )
        .expect(HasError("it"));
    }

    #[test]
    fn it_used_multi_param_context_error() {
        // Error: `it` used but arity is 2
        Test::new(
            r#"
module Main

func test() -> (Int, Int) -> Int {
    { it }
}
"#,
        )
        .expect(HasError("it"));
    }

    #[test]
    fn it_not_in_scope_with_explicit_params() {
        // Error: `it` not in scope when explicit params declared
        Test::new(
            r#"
module Main

func test() -> (Int) -> Int {
    { (x) in it }
}
"#,
        )
        .expect(HasError("it"));
    }

    #[test]
    fn it_shadowed_in_nested_closure() {
        // Inner `it` shadows outer `it`
        Test::new(
            r#"
module Main

func apply(f: (Int) -> Int) -> Int {
    f(10)
}

func test() -> (Int) -> Int {
    {
        let outer = it;
        apply({ it + outer })
    }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// MULTI-STATEMENT CLOSURES
// ============================================================================

mod multi_statement {
    use super::*;

    #[test]
    fn closure_with_let_binding() {
        // Closure with local variable
        Test::new(
            r#"
module Main

func test() -> (Int) -> Int {
    { (x) in
        let y = x * 2;
        y
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_with_multiple_statements() {
        // Multiple statements, last expression returned
        Test::new(
            r#"
module Main

func test() -> (Int, Int) -> Int {
    { (x, y) in
        let sum = x + y;
        let doubled = sum * 2;
        let result = doubled + 1;
        result
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_with_var_binding() {
        // Closure with mutable local variable
        Test::new(
            r#"
module Main

func test() -> (Int) -> Int {
    { (x) in
        var acc = 0;
        acc = acc + x;
        acc = acc + x;
        acc
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_with_if_expression() {
        // Closure containing if expression
        Test::new(
            r#"
module Main

func test() -> (Int) -> Int {
    { (x) in
        if x > 0 {
            x
        } else {
            0 - x
        }
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_with_while_loop() {
        // Closure containing while loop
        Test::new(
            r#"
module Main

func test() -> (Int) -> Int {
    { (n) in
        var i = 0;
        var sum = 0;
        while i < n {
            sum = sum + i;
            i = i + 1;
        }
        sum
    }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// CAPTURES
// ============================================================================

mod captures {
    use super::*;

    #[test]
    fn capture_immutable_variable() {
        // Capture a let-bound variable
        Test::new(
            r#"
module Main

func test() -> () -> Int {
    let x = 10;
    { x + 1 }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn capture_mutable_variable_read_only() {
        // Capture a var-bound variable (read only)
        Test::new(
            r#"
module Main

func test() -> () -> Int {
    var x = 10;
    { x + 1 }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn capture_multiple_variables() {
        // Capture multiple variables
        Test::new(
            r#"
module Main

func test() -> () -> Int {
    let a = 1;
    let b = 2;
    let c = 3;
    { a + b + c }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn capture_function_parameter() {
        // Capture function parameter
        Test::new(
            r#"
module Main

func test(multiplier: Int) -> (Int) -> Int {
    { it * multiplier }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn capture_from_nested_scope() {
        // Capture from outer scope through nested scopes
        Test::new(
            r#"
module Main

func test() -> () -> Int {
    let outer = 100;
    if true {
        let inner = 10;
        { outer + inner }
    } else {
        { outer }
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn cannot_mutate_captured_variable() {
        // Error: cannot assign to captured variable
        Test::new(
            r#"
module Main

func test() -> () -> Int {
    var x = 10;
    {
        x = 20;
        x
    }
}
"#,
        )
        .expect(HasError("cannot assign"));
    }

    #[test]
    fn capture_by_value_semantics() {
        // Captures are by value - original mutation doesn't affect capture
        // This tests that captures happen at closure creation time
        Test::new(
            r#"
module Main

func test() -> () -> Int {
    var x = 10;
    let f = { x };
    x = 20;
    f
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// CLOSURE PARAMETERS MUTABILITY
// ============================================================================

mod param_mutability {
    use super::*;

    #[test]
    fn closure_param_immutable_by_default() {
        // Cannot assign to closure parameter
        Test::new(
            r#"
module Main

func test() -> (Int) -> Int {
    { (x) in
        x = 10;
        x
    }
}
"#,
        )
        .expect(HasError("cannot assign"));
    }
}

// ============================================================================
// TRAILING CLOSURE SYNTAX
// ============================================================================

mod trailing_closure {
    use super::*;

    #[test]
    fn trailing_closure_only_argument() {
        // Closure as only argument, trailing syntax
        Test::new(
            r#"
module Main

func apply(f: () -> Int) -> Int {
    f()
}

func test() -> Int {
    apply { 42 }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn trailing_closure_with_other_args() {
        // Trailing closure with other arguments before it
        Test::new(
            r#"
module Main

func fold(initial: Int, f: (Int, Int) -> Int) -> Int {
    f(initial, 10)
}

func test() -> Int {
    fold(0) { (acc, n) in acc + n }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn trailing_closure_with_multiple_args() {
        // Multiple args before trailing closure
        Test::new(
            r#"
module Main

func combine(a: Int, b: Int, f: (Int) -> Int) -> Int {
    f(a + b)
}

func test() -> Int {
    combine(1, 2) { it * 2 }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn non_trailing_closure_in_parens() {
        // Closure as argument inside parentheses
        Test::new(
            r#"
module Main

func apply(f: (Int) -> Int) -> Int {
    f(10)
}

func test() -> Int {
    apply({ it * 2 })
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
    fn infer_param_type_from_expected() {
        // Parameter type inferred from expected function type
        Test::new(
            r#"
module Main

func test() -> (Int) -> Int {
    { (x) in x + 1 }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_return_type_from_body() {
        // Return type inferred from body expression
        Test::new(
            r#"
module Main

func test() -> (Int) -> Int {
    { (x: Int) in x * 2 }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_from_function_parameter_context() {
        // Infer closure type from function parameter
        Test::new(
            r#"
module Main

func transform(x: Int, f: (Int) -> Int) -> Int {
    f(x)
}

func test() -> Int {
    transform(5, { it * 2 })
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_from_variable_annotation() {
        // Infer closure type from variable type annotation
        Test::new(
            r#"
module Main

func test() {
    let f: (Int, Int) -> Int = { (a, b) in a + b };
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn cannot_infer_without_context_error() {
        // Error: cannot infer type without context
        Test::new(
            r#"
module Main

func test() {
    let f = { (x) in x };
}
"#,
        )
        .expect(HasError("could not infer type"));
    }

    #[test]
    fn cannot_infer_it_type_without_context() {
        // Error: cannot infer `it` type without context
        Test::new(
            r#"
module Main

func test() {
    let f = { it + 1 };
}
"#,
        )
        .expect(HasError("could not infer type"));
    }
}

// ============================================================================
// CLOSURE TYPE CHECKING
// ============================================================================

mod type_checking {
    use super::*;

    #[test]
    fn closure_arity_mismatch_too_few() {
        // Error: closure has fewer params than expected
        Test::new(
            r#"
module Main

func test() -> (Int, Int) -> Int {
    { (x) in x }
}
"#,
        )
        .expect(HasError(""));
    }

    #[test]
    fn closure_arity_mismatch_too_many() {
        // Error: closure has more params than expected
        Test::new(
            r#"
module Main

func test() -> (Int) -> Int {
    { (x, y) in x + y }
}
"#,
        )
        .expect(HasError(""));
    }

    #[test]
    fn closure_return_type_mismatch() {
        // Error: closure returns wrong type
        Test::new(
            r#"
module Main

func test() -> (Int) -> String {
    { (x) in x * 2 }
}
"#,
        )
        .expect(HasError(""));
    }

    #[test]
    fn closure_param_type_mismatch() {
        // Error: explicit param type doesn't match expected
        Test::new(
            r#"
module Main

func test() -> (Int) -> Int {
    { (x: String) in 42 }
}
"#,
        )
        .expect(HasError(""));
    }

    #[test]
    fn closure_assigned_to_non_function_type() {
        // Error: closure assigned to non-function type
        Test::new(
            r#"
module Main

func test() {
    let x: Int = { 42 };
}
"#,
        )
        .expect(HasError(""));
    }
}

// ============================================================================
// IMMEDIATE INVOCATION
// ============================================================================

mod immediate_invocation {
    use super::*;

    #[test]
    fn immediately_invoked_no_params() {
        // Immediately invoked closure with no params
        Test::new(
            r#"
module Main

func test() -> Int {
    { 42 }()
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn immediately_invoked_with_args() {
        // Immediately invoked closure with arguments
        Test::new(
            r#"
module Main

func test() -> Int {
    { (x: Int, y: Int) in x + y }(10, 20)
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn immediately_invoked_for_scoping() {
        // Use immediate invocation for local scoping
        Test::new(
            r#"
module Main

func test() -> Int {
    let result = {
        let a = 10;
        let b = 20;
        a + b
    }();
    result
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn immediately_invoked_wrong_arg_count() {
        // Error: wrong number of args to immediately invoked closure
        Test::new(
            r#"
module Main

func test() -> Int {
    { (x: Int) in x }(1, 2)
}
"#,
        )
        .expect(HasError(""));
    }
}

// ============================================================================
// CLOSURES AS VALUES
// ============================================================================

mod closures_as_values {
    use super::*;

    #[test]
    fn closure_stored_in_variable() {
        // Store closure in variable and call later
        Test::new(
            r#"
module Main

func test() -> Int {
    let f: (Int) -> Int = { it * 2 };
    f(21)
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_passed_to_function() {
        // Pass closure as argument to function
        Test::new(
            r#"
module Main

func apply(x: Int, f: (Int) -> Int) -> Int {
    f(x)
}

func test() -> Int {
    apply(10, { it + 1 })
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_returned_from_function() {
        // Return closure from function
        Test::new(
            r#"
module Main

func makeAdder(n: Int) -> (Int) -> Int {
    { it + n }
}

func test() -> Int {
    let add5 = makeAdder(5);
    add5(10)
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_in_struct_field() {
        // Store closure in struct field
        Test::new(
            r#"
module Main

struct Callback {
    let action: () -> Int
}

func test() -> Int {
    let cb = Callback(action: { 42 });
    (cb.action)()
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_in_generic_struct() {
        // Closure in generic struct field
        Test::new(
            r#"
module Main

struct Handler[T] {
    let handle: (T) -> T
}

func test() -> Int {
    let h = Handler[Int](handle: { it * 2 });
    (h.handle)(21)
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// NESTED CLOSURES
// ============================================================================

mod nested_closures {
    use super::*;

    #[test]
    fn closure_returning_closure() {
        // Closure that returns another closure
        Test::new(
            r#"
module Main

func test() -> (Int) -> (Int) -> Int {
    { (x) in { (y) in x + y } }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn nested_closure_captures_outer_param() {
        // Inner closure captures outer closure's parameter
        Test::new(
            r#"
module Main

func test() -> Int {
    let f: (Int) -> (Int) -> Int = { (x) in { (y) in x + y } };
    let add10 = f(10);
    add10(5)
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn deeply_nested_closures() {
        // Three levels of nested closures
        Test::new(
            r#"
module Main

func test() -> (Int) -> (Int) -> (Int) -> Int {
    { (a) in { (b) in { (c) in a + b + c } } }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn nested_closure_with_it_shadowing() {
        // Inner closure's `it` shadows outer
        Test::new(
            r#"
module Main

func apply(f: (Int) -> Int) -> Int {
    f(5)
}

func test() -> (Int) -> Int {
    {
        let outer = it;
        apply({ it + outer })
    }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// CLOSURES WITH GENERICS
// ============================================================================

mod generics {
    use super::*;

    #[test]
    fn closure_in_generic_function() {
        // Closure inside generic function captures type param
        Test::new(
            r#"
module Main

func identity[T](x: T, f: (T) -> T) -> T {
    f(x)
}

func test() -> Int {
    identity(10, { it })
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_with_generic_param_inferred() {
        // Closure parameter type inferred from generic context
        Test::new(
            r#"
module Main

func transform[T, U](x: T, f: (T) -> U) -> U {
    f(x)
}

func test() -> String {
    transform(42, { (n) in "hello" })
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// EDGE CASES
// ============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn closure_returning_unit() {
        // Closure with unit return type
        Test::new(
            r#"
module Main

func test() -> () -> () {
    { () }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_with_never_return() {
        // Closure that never returns (contains return statement)
        Test::new(
            r#"
module Main

func earlyReturn(f: () -> Int) -> Int {
    f()
}

func test() -> Int {
    earlyReturn({
        return 42
    })
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn empty_closure_body() {
        // Closure with empty body returns unit
        Test::new(
            r#"
module Main

func test() -> () -> () {
    { }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_with_only_statements() {
        // Closure with only statements, no tail expression
        Test::new(
            r#"
module Main

func test() -> () -> () {
    { 
        let x = 1;
        let y = 2;
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_single_expression_is_tail() {
        // Single expression without semicolon is tail
        Test::new(
            r#"
module Main

func test() -> () -> Int {
    { 42 }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn closure_expression_with_semicolon_returns_unit() {
        // Expression with semicolon is statement, returns unit
        Test::new(
            r#"
module Main

func test() -> () -> () {
    { 42; }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn param_named_it_shadows_implicit() {
        // Explicit param named `it` works
        Test::new(
            r#"
module Main

func test() -> (Int) -> Int {
    { (it) in it * 2 }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn variable_named_it_in_closure() {
        // Local variable named `it` shadows implicit
        Test::new(
            r#"
module Main

func test() -> (Int) -> Int {
    {
        let it = 100;
        it
    }
}
"#,
        )
        .expect(Compiles);
    }
}
