//! Tests for parameter access modes: borrow (default), mutating, consuming.
//!
//! These tests verify the Phase 1 implementation of the memory model.

use kestrel_test_suite::*;

// =============================================================================
// PARSING TESTS - Verify the syntax is accepted
// =============================================================================

mod parsing {
    use super::*;

    #[test]
    fn borrow_parameter_default() {
        // Default access mode is borrow (no keyword)
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func readPoint(p: Point) -> Int {
                p.x
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("readPoint")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn mutating_parameter() {
        // mutating keyword before parameter name
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func reset(mutating p: Point) {
                p.x = 0;
                p.y = 0;
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("reset")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn consuming_parameter() {
        // consuming keyword before parameter name
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func consume(consuming p: Point) -> Int {
                p.x
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("consume")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn mutating_with_label() {
        // mutating with external label (access mode comes before label)
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func reset(mutating point p: Point) {
                p.x = 0;
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("reset")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn consuming_with_label() {
        // consuming with external label (access mode comes before label)
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func take(consuming point p: Point) -> Int {
                p.x
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("take")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn multiple_parameters_mixed_modes() {
        // Mix of access modes in one function
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func process(a: Point, mutating b: Point, consuming c: Point) -> Int {
                b.x = a.x;
                c.x
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("process")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(3)),
        );
    }

    #[test]
    fn mutating_consuming_combined_is_error() {
        // Cannot combine mutating and consuming
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func bad(mutating consuming p: Point) {}
        "#,
        )
        .expect(Fails);
    }
}

// =============================================================================
// BORROW (DEFAULT) TESTS
// =============================================================================

mod borrow_mode {
    use super::*;

    #[test]
    fn borrow_allows_read() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func readX(p: Point) -> Int {
                p.x
            }
            func test() -> Int {
                let p = Point(x: 1, y: 2);
                readX(p)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn borrow_disallows_write() {
        // Cannot modify a borrowed parameter
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func tryModify(p: Point) {
                p.x = 10
            }
        "#,
        )
        .expect(HasError("cannot assign to immutable"));
    }

    #[test]
    fn borrow_caller_retains_value() {
        // After passing to borrow, caller can still use the value
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func readX(p: Point) -> Int { p.x }
            func test() -> Int {
                let p = Point(x: 1, y: 2);
                let _ = readX(p);
                p.x
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn borrow_accepts_let_binding() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func readX(p: Point) -> Int { p.x }
            func test() -> Int {
                let p = Point(x: 1, y: 2);
                readX(p)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn borrow_accepts_var_binding() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func readX(p: Point) -> Int { p.x }
            func test() -> Int {
                var p = Point(x: 1, y: 2);
                readX(p)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn borrow_accepts_temporary() {
        // Temporaries can be borrowed
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func readX(p: Point) -> Int { p.x }
            func test() -> Int {
                readX(Point(x: 1, y: 2))
            }
        "#,
        )
        .expect(Compiles);
    }
}

// =============================================================================
// MUTATING PARAMETER TESTS
// =============================================================================

mod mutating_mode {
    use super::*;

    #[test]
    fn mutating_allows_write() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func reset(mutating p: Point) {
                p.x = 0;
                p.y = 0;
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn mutating_allows_read() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func double(mutating p: Point) {
                p.x = p.x * 2;
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn mutating_with_var_binding() {
        // Passing a var binding to mutating parameter should work
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func reset(mutating p: Point) {
                p.x = 0;
            }
            func test() {
                var p = Point(x: 1, y: 2);
                reset(p)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn mutating_with_let_binding_fails() {
        // Cannot pass a let binding to mutating parameter
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func reset(mutating p: Point) {
                p.x = 0;
            }
            func test() {
                let p = Point(x: 1, y: 2);
                reset(p)
            }
        "#,
        )
        .expect(HasError("mutating"));
    }

    #[test]
    fn mutating_with_temporary_fails() {
        // Cannot pass a temporary to mutating parameter
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func reset(mutating p: Point) {
                p.x = 0;
            }
            func test() {
                reset(Point(x: 1, y: 2))
            }
        "#,
        )
        .expect(HasError("mutating"));
    }

    #[test]
    fn mutating_with_mutable_field() {
        // Can pass a mutable field to mutating parameter
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            struct Container { var point: Point }
            func reset(mutating p: Point) {
                p.x = 0;
            }
            func test() {
                var c = Container(point: Point(x: 1, y: 2));
                reset(c.point)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn mutating_with_immutable_field_fails() {
        // Cannot pass an immutable field to mutating parameter
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            struct Container { let point: Point }
            func reset(mutating p: Point) {
                p.x = 0;
            }
            func test() {
                var c = Container(point: Point(x: 1, y: 2));
                reset(c.point)
            }
        "#,
        )
        .expect(HasError("mutating"));
    }

    #[test]
    fn mutating_through_let_binding_fails() {
        // Cannot pass field of let binding to mutating parameter
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            struct Container { var point: Point }
            func reset(mutating p: Point) {
                p.x = 0;
            }
            func test() {
                let c = Container(point: Point(x: 1, y: 2));
                reset(c.point)
            }
        "#,
        )
        .expect(HasError("mutating"));
    }

    #[test]
    fn mutating_nested_field_all_mutable() {
        // Can pass deeply nested mutable field
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            struct Inner { var point: Point }
            struct Outer { var inner: Inner }
            func reset(mutating p: Point) {
                p.x = 0;
            }
            func test() {
                var o = Outer(inner: Inner(point: Point(x: 1, y: 2)));
                reset(o.inner.point)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn mutating_nested_field_middle_immutable_fails() {
        // Cannot pass if any field in chain is immutable
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            struct Inner { var point: Point }
            struct Outer { let inner: Inner }
            func reset(mutating p: Point) {
                p.x = 0;
            }
            func test() {
                var o = Outer(inner: Inner(point: Point(x: 1, y: 2)));
                reset(o.inner.point)
            }
        "#,
        )
        .expect(HasError("mutating"));
    }

    #[test]
    fn mutating_caller_sees_changes() {
        // Modifications through mutating parameter are visible to caller
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func reset(mutating p: Point) {
                p.x = 0;
            }
            func test() -> Int {
                var p = Point(x: 1, y: 2);
                reset(p);
                p.x
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn mutating_primitive_with_var() {
        // Primitives can also be mutating parameters
        Test::new(
            r#"module Test
            func increment(mutating n: Int) {
                n = n + 1;
            }
            func test() {
                var x = 5;
                increment(x)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn mutating_primitive_with_let_fails() {
        Test::new(
            r#"module Test
            func increment(mutating n: Int) {
                n = n + 1;
            }
            func test() {
                let x = 5;
                increment(x)
            }
        "#,
        )
        .expect(HasError("mutating"));
    }
}

// =============================================================================
// CONSUMING PARAMETER TESTS
// =============================================================================

mod consuming_mode {
    use super::*;

    #[test]
    fn consuming_allows_read() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func consume(consuming p: Point) -> Int {
                p.x
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn consuming_parameter_is_mutable() {
        // Consuming parameters are mutable inside the function body
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func transform(consuming p: Point) -> Point {
                p.x = p.x * 2;
                p.y = p.y * 2;
                p
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn consuming_with_let_binding() {
        // Can pass let binding to consuming (for Copyable types, a copy is made)
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func consume(consuming p: Point) -> Int {
                p.x
            }
            func test() -> Int {
                let p = Point(x: 1, y: 2);
                consume(p)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn consuming_with_var_binding() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func consume(consuming p: Point) -> Int {
                p.x
            }
            func test() -> Int {
                var p = Point(x: 1, y: 2);
                consume(p)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn consuming_with_temporary() {
        // Temporaries can be consumed
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func consume(consuming p: Point) -> Int {
                p.x
            }
            func test() -> Int {
                consume(Point(x: 1, y: 2))
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn consuming_caller_can_still_use_copyable() {
        // For Copyable types, caller can still use the value after consuming
        // (because a copy was passed)
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func consume(consuming p: Point) -> Int {
                p.x
            }
            func test() -> Int {
                let p = Point(x: 1, y: 2);
                let _ = consume(p);
                p.x
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn consuming_primitive() {
        Test::new(
            r#"module Test
            func take(consuming n: Int) -> Int {
                n * 2
            }
            func test() -> Int {
                let x = 5;
                take(x)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn consuming_can_reassign_parameter() {
        // Consuming parameter can be reassigned
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func replace(consuming p: Point) -> Point {
                p = Point(x: 0, y: 0);
                p
            }
        "#,
        )
        .expect(Compiles);
    }
}

// =============================================================================
// METHOD RECEIVER INTERACTION TESTS
// =============================================================================

mod method_interaction {
    use super::*;

    #[test]
    fn mutating_method_with_mutating_param() {
        // A mutating method can call a function with mutating parameter on its field
        // This tests forward references: reset is declared AFTER Shape
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            struct Shape {
                var origin: Point
                
                mutating func resetOrigin() {
                    reset(self.origin)
                }
            }
            func reset(mutating p: Point) {
                p.x = 0;
                p.y = 0;
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn non_mutating_method_cannot_pass_field_to_mutating() {
        // A non-mutating method cannot pass its field to a mutating parameter
        // This tests forward references: reset is declared AFTER Shape
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            struct Shape {
                var origin: Point
                
                func tryReset() {
                    reset(self.origin)
                }
            }
            func reset(mutating p: Point) {
                p.x = 0;
            }
        "#,
        )
        .expect(HasError("mutating"));
    }

    #[test]
    fn consuming_method_with_consuming_param() {
        // A consuming method can pass self.field to consuming parameter
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            struct Container {
                var point: Point
                
                consuming func takePoint() -> Int {
                    consume(self.point)
                }
            }
            func consume(consuming p: Point) -> Int {
                p.x
            }
        "#,
        )
        .expect(Compiles);
    }
}

// =============================================================================
// EDGE CASES AND ERROR MESSAGES
// =============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn function_call_result_to_mutating_fails() {
        // Cannot pass function call result to mutating
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func makePoint() -> Point {
                Point(x: 1, y: 2)
            }
            func reset(mutating p: Point) {
                p.x = 0;
            }
            func test() {
                reset(makePoint())
            }
        "#,
        )
        .expect(HasError("mutating"));
    }

    #[test]
    fn if_expression_result_to_mutating_fails() {
        // Cannot pass if expression result to mutating
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func reset(mutating p: Point) {
                p.x = 0;
            }
            func test(cond: Bool) {
                reset(if cond { Point(x: 1, y: 2) } else { Point(x: 3, y: 4) })
            }
        "#,
        )
        .expect(HasError("mutating"));
    }

    #[test]
    fn tuple_field_to_mutating() {
        // Can pass tuple field to mutating if tuple is mutable
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func reset(mutating p: Point) {
                p.x = 0;
            }
            func test() {
                var tuple = (Point(x: 1, y: 2), 42);
                reset(tuple.0)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn tuple_field_let_to_mutating_fails() {
        // Cannot pass tuple field to mutating if tuple is immutable
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func reset(mutating p: Point) {
                p.x = 0;
            }
            func test() {
                let tuple = (Point(x: 1, y: 2), 42);
                reset(tuple.0)
            }
        "#,
        )
        .expect(HasError("mutating"));
    }
}

// =============================================================================
// MIR TESTS - Verify passing modes are emitted in MIR
// =============================================================================

mod mir_passing_modes {
    use super::*;
    use kestrel_test_suite::mir::*;

    #[test]
    fn mir_call_has_passing_modes() {
        // Verify that calls in MIR include passing mode information
        // For now, all arguments default to Ref (borrow)
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            func process(p: Point) -> Int {
                p.x
            }
            func caller() -> Int {
                let pt = Point(x: 1, y: 2);
                process(pt)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Test.caller")
                .any_block(|b| {
                    b.has_statement(StatementPattern::CallWithModes {
                        callee: "Test.process".to_string(),
                        arg_modes: vec![PassingMode::Ref],
                    })
                }),
        );
    }

    #[test]
    fn mir_method_call_has_passing_modes() {
        // Method calls also have passing modes (self arg is Ref by default)
        Test::new(
            r#"module Test
            struct Point { 
                var x: Int
                var y: Int 
                
                func magnitude() -> Int {
                    self.x + self.y
                }
            }
            func caller() -> Int {
                let pt = Point(x: 1, y: 2);
                pt.magnitude()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Test.caller")
                .any_block(|b| {
                    b.has_statement(StatementPattern::CallWithModes {
                        callee: "Test.Point.magnitude".to_string(),
                        arg_modes: vec![PassingMode::Ref], // self is Ref
                    })
                }),
        );
    }
}
