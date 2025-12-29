//! Basic MIR tests for functions, arithmetic, and simple control flow.

use kestrel_test_suite::mir::*;
use kestrel_test_suite::*;

/// Based on tmp/01_basic_functions.ks
mod basic_functions {
    use super::*;

    #[test]
    fn function_returns_literal() {
        Test::new(
            r#"
            module Main
            func answer() -> Int { 42 }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_function("Main.answer").returns(MirTy::I64).has_param_count(0));
    }

    #[test]
    fn function_with_parameters() {
        Test::new(
            r#"
            module Main
            func add(a: Int, b: Int) -> Int { a + b }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.add")
                .returns(MirTy::I64)
                .has_param("a", MirTy::I64)
                .has_param("b", MirTy::I64)
                .has_param_count(2),
        );
    }

    #[test]
    fn function_with_arithmetic() {
        Test::new(
            r#"
            module Main
            func calculate(x: Int, y: Int) -> Int {
                let sum = x + y;
                let product = x * y;
                sum + product
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.calculate")
                .returns(MirTy::I64)
                .has_param_count(2)
                .has_local("sum", MirTy::I64)
                .has_local("product", MirTy::I64)
                .any_block(|b| b.has_statement(StatementPattern::BinOp(BinOp::AddSigned)))
                .any_block(|b| b.has_statement(StatementPattern::BinOp(BinOp::MulSigned))),
        );
    }

    #[test]
    fn function_count() {
        Test::new(
            r#"
            module Main
            func one() -> Int { 1 }
            func two() -> Int { 2 }
            func three() -> Int { 3 }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::function_count(3));
    }
}

/// Based on tmp/02_control_flow.ks
mod control_flow {
    use super::*;

    #[test]
    fn if_else_creates_branches() {
        Test::new(
            r#"
            module Main
            func abs(x: Int) -> Int {
                if x < 0 { -x } else { x }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.abs")
                .returns(MirTy::I64)
                .has_at_least_blocks(3) // entry, then, else (maybe join)
                .any_block(|b| b.terminates_with(TerminatorPattern::Branch)),
        );
    }

    #[test]
    fn if_else_has_comparison() {
        Test::new(
            r#"
            module Main
            func max(a: Int, b: Int) -> Int {
                if a > b { a } else { b }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.max")
                .any_block(|b| b.has_statement(StatementPattern::BinOp(BinOp::GtSigned))),
        );
    }
}

/// Struct tests based on tmp/03_structs.ks
mod structs {
    use super::*;

    #[test]
    fn struct_definition() {
        Test::new(
            r#"
            module Main
            struct Point {
                let x: Int
                let y: Int
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_struct("Main.Point")
                .has_field("x", MirTy::I64)
                .has_field("y", MirTy::I64)
                .has_field_count(2),
        );
    }

    #[test]
    fn struct_method() {
        Test::new(
            r#"
            module Main
            struct Point {
                let x: Int
                let y: Int
                
                func distanceSquared() -> Int {
                    self.x * self.x + self.y * self.y
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.Point.distanceSquared")
                .returns(MirTy::I64)
                .has_param("self", MirTy::ref_(MirTy::named("Main.Point"))),
        );
    }

    #[test]
    fn struct_construction() {
        Test::new(
            r#"
            module Main
            struct Point {
                let x: Int
                let y: Int
            }
            
            func makePoint() -> Point {
                Point(x: 3, y: 4)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.makePoint")
                .returns(MirTy::named("Main.Point"))
                .any_block(|b| {
                    b.has_statement(StatementPattern::Construct {
                        ty: "Main.Point".to_string(),
                    })
                }),
        );
    }
}

/// Enum tests based on tmp/04_enums.ks
mod enums {
    use super::*;

    #[test]
    fn simple_enum() {
        Test::new(
            r#"
            module Main
            enum Color {
                case Red
                case Green
                case Blue
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_enum("Main.Color")
                .has_case("Red")
                .has_case("Green")
                .has_case("Blue")
                .has_case_count(3),
        );
    }

    #[test]
    fn enum_match_creates_switch() {
        Test::new(
            r#"
            module Main
            enum Color {
                case Red
                case Green
                case Blue
            }
            
            func toInt(c: Color) -> Int {
                match c {
                    .Red => 1,
                    .Green => 2,
                    .Blue => 3
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.toInt").any_block(|b| b.terminates_with(TerminatorPattern::Switch)),
        );
    }

    #[test]
    fn enum_with_payload() {
        Test::new(
            r#"
            module Main
            enum Option {
                case Some(value: Int)
                case None
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_enum("Main.Option")
                .has_case("Some")
                .has_case("None")
                .has_case_count(2),
        );
    }
}
