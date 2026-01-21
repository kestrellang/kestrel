//! Enum MIR tests.
//!
//! Tests for enum lowering including:
//! - Simple enums (no payloads)
//! - Enums with payloads
//! - Enum construction
//! - Match expressions on enums
//! - Recursive enums (indirect)
//! - Enum methods

use kestrel_test_suite::mir::*;
use kestrel_test_suite::*;

// ============================================================================
// SIMPLE ENUMS
// ============================================================================

mod simple_enums {
    use super::*;

    #[test]
    fn unit_enum() {
        Test::new(
            r#"
            module Main

            enum Direction {
                case North
                case South
                case East
                case West
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_enum("Main.Direction")
                .has_case("North")
                .has_case("South")
                .has_case("East")
                .has_case("West")
                .has_case_count(4),
        );
    }

    #[test]
    fn color_enum() {
        // Based on tmp/04_enums.ks
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
}

// ============================================================================
// ENUMS WITH PAYLOADS
// ============================================================================

mod enums_with_payloads {
    use super::*;

    #[test]
    fn option_enum() {
        Test::new(
            r#"
            module Main

            enum Option {
                case Some(value: lang.i64)
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

    #[test]
    fn result_enum() {
        Test::new(
            r#"
            module Main

            enum Result {
                case Ok(value: lang.i64)
                case Err(message: lang.str)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_enum("Main.Result")
                .has_case("Ok")
                .has_case("Err")
                .has_case_count(2),
        );
    }

    #[test]
    fn enum_with_multiple_payload_fields() {
        Test::new(
            r#"
            module Main

            enum Shape {
                case Circle(x: lang.i64, y: lang.i64, radius: lang.i64)
                case Rectangle(x: lang.i64, y: lang.i64, width: lang.i64, height: lang.i64)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_enum("Main.Shape")
                .has_case("Circle")
                .has_case("Rectangle")
                .has_case_count(2),
        );
    }
}

// ============================================================================
// ENUM CONSTRUCTION
// ============================================================================

mod enum_construction {
    use super::*;

    #[test]
    fn construct_unit_case() {
        Test::new(
            r#"
            module Main

            enum Option {
                case Some(value: lang.i64)
                case None
            }

            func makeNone() -> Option {
                Option.None
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_function("Main.makeNone").returns(MirTy::named("Main.Option")));
    }

    #[test]
    fn construct_case_with_payload() {
        // Note: Parameters default to borrow mode
        Test::new(
            r#"
            module Main

            enum Option {
                case Some(value: lang.i64)
                case None
            }

            func makeSome(x: lang.i64) -> Option {
                Option.Some(value: x)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.makeSome")
                .returns(MirTy::named("Main.Option"))
                .has_param("x", MirTy::ref_(MirTy::I64)),
        );
    }

    #[test]
    fn construct_and_return_directly() {
        Test::new(
            r#"
            module Main

            enum Color {
                case Red
                case Green
                case Blue
            }

            func getRed() -> Color { Color.Red }
            func getGreen() -> Color { Color.Green }
            func getBlue() -> Color { Color.Blue }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::function_count(3));
    }
}

// ============================================================================
// MATCH ON ENUMS
// ============================================================================

mod match_on_enums {
    use super::*;

    #[test]
    fn match_unit_cases() {
        // Based on tmp/04_enums.ks
        Test::new(
            r#"
            module Main

            enum Color {
                case Red
                case Green
                case Blue
            }
            
            func toInt(c: Color) -> lang.i64 {
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
            Mir::mir_function("Main.toInt")
                .returns(MirTy::I64)
                .any_block(|b| b.terminates_with(TerminatorPattern::Switch)),
        );
    }

    #[test]
    fn match_with_payload_binding() {
        // Note: Parameters default to borrow mode
        Test::new(
            r#"
            module Main

            enum Option {
                case Some(value: lang.i64)
                case None
            }

            func unwrapOr(opt: Option, default: lang.i64) -> lang.i64 {
                match opt {
                    .Some(value) => value,
                    .None => default
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.unwrapOr")
                .returns(MirTy::I64)
                .has_param("opt", MirTy::ref_(MirTy::named("Main.Option")))
                .has_param("default", MirTy::ref_(MirTy::I64))
                .any_block(|b| b.terminates_with(TerminatorPattern::Switch)),
        );
    }

    #[test]
    fn match_with_multiple_bindings() {
        Test::new(
            r#"
            module Main

            enum Shape {
                case Circle(x: lang.i64, y: lang.i64, radius: lang.i64)
                case Point(x: lang.i64, y: lang.i64)
            }

            func getX(s: Shape) -> lang.i64 {
                match s {
                    .Circle(x, y, radius) => x,
                    .Point(x, y) => x
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.getX")
                .returns(MirTy::I64)
                .any_block(|b| b.terminates_with(TerminatorPattern::Switch)),
        );
    }
}

// ============================================================================
// RECURSIVE ENUMS
// ============================================================================

mod recursive_enums {
    use super::*;

    #[test]
    fn binary_tree() {
        // Based on tmp/12_recursive_enum.ks
        Test::new(
            r#"
            module Main

            indirect enum Tree {
                case Leaf(value: lang.i64)
                case Node(left: Tree, right: Tree)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_enum("Main.Tree")
                .has_case("Leaf")
                .has_case("Node")
                .has_case_count(2),
        );
    }

    #[test]
    fn recursive_tree_sum() {
        // Based on tmp/12_recursive_enum.ks
        Test::new(
            r#"
            module Main

            indirect enum Tree {
                case Leaf(value: lang.i64)
                case Node(left: Tree, right: Tree)
            }

            func sum(tree: Tree) -> lang.i64 {
                match tree {
                    .Leaf(value) => value,
                    .Node(left, right) => lang.i64_add(sum(left), sum(right))
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.sum")
                .returns(MirTy::I64)
                .calls("Main.sum") // Recursive call
                .any_block(|b| b.terminates_with(TerminatorPattern::Switch)),
        );
    }

    #[test]
    fn linked_list() {
        Test::new(
            r#"
            module Main

            indirect enum List {
                case Cons(head: lang.i64, tail: List)
                case Nil
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_enum("Main.List")
                .has_case("Cons")
                .has_case("Nil")
                .has_case_count(2),
        );
    }
}

// ============================================================================
// ENUM WITH STRUCT PAYLOADS
// ============================================================================

mod enum_struct_payloads {
    use super::*;

    #[test]
    fn enum_with_struct_payload() {
        // Based on tmp/27_enum_with_struct_payload.ks
        Test::new(
            r#"
            module Main

            struct Point {
                let x: lang.i64
                let y: lang.i64
            }

            struct Size {
                let width: lang.i64
                let height: lang.i64
            }

            enum Shape {
                case Circle(center: Point, radius: lang.i64)
                case Rectangle(origin: Point, size: Size)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_enum("Main.Shape")
                .has_case("Circle")
                .has_case("Rectangle")
                .has_case_count(2),
        )
        .expect(Mir::struct_count(6)); // Point, Size, 2 enum case structs, and 2 Prelude.ControlFlow case structs
    }

    #[test]
    fn match_struct_payload() {
        // Based on tmp/54_enum_method_dispatch.ks
        Test::new(
            r#"
            module Main

            struct Circle {
                let radius: lang.i64

                func area() -> lang.i64 {
                    lang.i64_mul(lang.i64_mul(self.radius, self.radius), 3)
                }
            }

            struct Rectangle {
                let width: lang.i64
                let height: lang.i64

                func area() -> lang.i64 {
                    lang.i64_mul(self.width, self.height)
                }
            }

            enum Shape {
                case CircleShape(c: Circle)
                case RectShape(r: Rectangle)
            }

            func getArea(s: Shape) -> lang.i64 {
                match s {
                    .CircleShape(c) => c.area(),
                    .RectShape(r) => r.area()
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.getArea")
                .returns(MirTy::I64)
                .calls("Main.Circle.area")
                .calls("Main.Rectangle.area")
                .any_block(|b| b.terminates_with(TerminatorPattern::Switch)),
        );
    }
}

// ============================================================================
// NESTED MATCH
// ============================================================================

mod nested_match {
    use super::*;

    #[test]
    fn nested_enum_match() {
        // Based on tmp/49_nested_enum_match.ks
        Test::new(
            r#"
            module Main

            enum Inner {
                case A(x: lang.i64)
                case B(y: lang.i64)
            }

            enum Outer {
                case Left(inner: Inner)
                case Right(inner: Inner)
            }

            func getValue(outer: Outer) -> lang.i64 {
                match outer {
                    .Left(inner) => match inner {
                        .A(x) => x,
                        .B(y) => y
                    },
                    .Right(inner) => match inner {
                        .A(x) => lang.i64_mul(x, 2),
                        .B(y) => lang.i64_mul(y, 2)
                    }
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.getValue")
                .returns(MirTy::I64)
                .has_at_least_blocks(5), // Multiple switch blocks for nested match
        );
    }
}

// ============================================================================
// WILDCARD PATTERNS IN MATCH
// ============================================================================

mod wildcard_patterns {
    use super::*;

    #[test]
    fn wildcard_pattern_in_match() {
        Test::new(
            r#"
            module Main

            enum Weekday {
                case Monday
                case Tuesday
                case Wednesday
                case Thursday
                case Friday
                case Saturday
                case Sunday
            }

            func isMonday(day: Weekday) -> lang.i1 {
                match day {
                    .Monday => true,
                    _ => false
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.isMonday")
                .returns(MirTy::Bool)
                .any_block(|b| b.terminates_with(TerminatorPattern::Switch)),
        );
    }
}
