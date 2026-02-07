//! Tests for protocol-based operator overloading
//!
//! Operators in Kestrel desugar to protocol method calls. This allows custom types
//! to implement operators by conforming to the appropriate operator protocols.
//!
//! Examples:
//! - `a + b` desugars to `a.add(b)` via `AddOperatorProtocol`
//! - `a - b` desugars to `a.subtract(b)` via `SubtractOperatorProtocol`
//! - `a == b` desugars to `a.equals(b)` via `EqualsOperatorProtocol`

use kestrel_test_suite::*;

mod arithmetic_protocols {
    use super::*;

    #[test]
    fn add_operator_protocol() {
        Test::new(
            r#"module Test
            struct Number: Prelude.AddOperatorProtocol {
                var value: lang.i64

                func add(rhs: Number) -> Number {
                    Number(value: lang.i64_add(self.value, rhs.value))
                }
            }
            func test() -> Number {
                let a = Number(value: 1);
                let b = Number(value: 2);
                a + b
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn subtract_operator_protocol() {
        Test::new(
            r#"module Test
            struct Number: Prelude.SubtractOperatorProtocol {
                var value: lang.i64

                func subtract(rhs: Number) -> Number {
                    Number(value: lang.i64_sub(self.value, rhs.value))
                }
            }
            func test() -> Number {
                let a = Number(value: 5);
                let b = Number(value: 3);
                a - b
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn multiply_operator_protocol() {
        Test::new(
            r#"module Test
            struct Number: Prelude.MultiplyOperatorProtocol {
                var value: lang.i64

                func multiply(rhs: Number) -> Number {
                    Number(value: lang.i64_mul(self.value, rhs.value))
                }
            }
            func test() -> Number {
                let a = Number(value: 3);
                let b = Number(value: 4);
                a * b
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn divide_operator_protocol() {
        Test::new(
            r#"module Test
            struct Number: Prelude.DivideOperatorProtocol {
                var value: lang.i64

                func divide(rhs: Number) -> Number {
                    Number(value: lang.i64_signed_div(self.value, rhs.value))
                }
            }
            func test() -> Number {
                let a = Number(value: 10);
                let b = Number(value: 2);
                a / b
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn remainder_operator_protocol() {
        Test::new(
            r#"module Test
            struct Number: Prelude.ModuloOperatorProtocol {
                var value: lang.i64

                func modulo(rhs: Number) -> Number {
                    Number(value: lang.i64_signed_rem(self.value, rhs.value))
                }
            }
            func test() -> Number {
                let a = Number(value: 10);
                let b = Number(value: 3);
                a % b
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod comparison_protocols {
    use super::*;

    #[test]
    fn equals_operator_protocol() {
        Test::new(
            r#"module Test
            struct Number: Prelude.EqualsOperatorProtocol {
                var value: lang.i64

                func equals(rhs: Number) -> lang.i1 {
                    lang.i64_eq(self.value, rhs.value)
                }
            }
            func test() -> lang.i1 {
                let a = Number(value: 5);
                let b = Number(value: 5);
                a == b
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn not_equals_operator_protocol() {
        Test::new(
            r#"module Test
            struct Number: Prelude.NotEqualsOperatorProtocol {
                var value: lang.i64

                func notEquals(rhs: Number) -> lang.i1 {
                    lang.i64_ne(self.value, rhs.value)
                }
            }
            func test() -> lang.i1 {
                let a = Number(value: 5);
                let b = Number(value: 3);
                a != b
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn less_than_operator_protocol() {
        Test::new(
            r#"module Test
            struct Number: Prelude.LessThanOperatorProtocol {
                var value: lang.i64

                func lessThan(rhs: Number) -> lang.i1 {
                    lang.i64_signed_lt(self.value, rhs.value)
                }
            }
            func test() -> lang.i1 {
                let a = Number(value: 3);
                let b = Number(value: 5);
                a < b
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn greater_than_operator_protocol() {
        Test::new(
            r#"module Test
            struct Number: Prelude.GreaterThanOperatorProtocol {
                var value: lang.i64

                func greaterThan(rhs: Number) -> lang.i1 {
                    lang.i64_signed_gt(self.value, rhs.value)
                }
            }
            func test() -> lang.i1 {
                let a = Number(value: 5);
                let b = Number(value: 3);
                a > b
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn less_than_or_equals_operator_protocol() {
        Test::new(
            r#"module Test
            struct Number: Prelude.LessOrEqualOperatorProtocol {
                var value: lang.i64

                func lessThanOrEqual(rhs: Number) -> lang.i1 {
                    lang.i64_signed_le(self.value, rhs.value)
                }
            }
            func test() -> lang.i1 {
                let a = Number(value: 3);
                let b = Number(value: 5);
                a <= b
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn greater_than_or_equals_operator_protocol() {
        Test::new(
            r#"module Test
            struct Number: Prelude.GreaterOrEqualOperatorProtocol {
                var value: lang.i64

                func greaterThanOrEqual(rhs: Number) -> lang.i1 {
                    lang.i64_signed_ge(self.value, rhs.value)
                }
            }
            func test() -> lang.i1 {
                let a = Number(value: 5);
                let b = Number(value: 3);
                a >= b
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod stdlib_comparison_protocols {
    use super::*;

    #[test]
    fn equatable_extension_binds_rhs_self_for_equals_operator() {
        Test::new(
            r#"module Test

            public enum LocalOrdering: std.core.Equatable {
                case Less
                case Equal

                public func equals(other: LocalOrdering) -> std.core.Bool {
                    true
                }
            }

            public func test() -> std.core.Bool {
                LocalOrdering.Less == LocalOrdering.Equal
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

mod bitwise_protocols {
    use super::*;

    #[test]
    fn bitwise_and_operator_protocol() {
        Test::new(
            r#"module Test
            struct Bits: Prelude.BitwiseAndOperatorProtocol {
                var value: lang.i64

                func bitwiseAnd(rhs: Bits) -> Bits {
                    Bits(value: lang.i64_and(self.value, rhs.value))
                }
            }
            func test() -> Bits {
                let a = Bits(value: 0b1100);
                let b = Bits(value: 0b1010);
                a & b
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn bitwise_or_operator_protocol() {
        Test::new(
            r#"module Test
            struct Bits: Prelude.BitwiseOrOperatorProtocol {
                var value: lang.i64

                func bitwiseOr(rhs: Bits) -> Bits {
                    Bits(value: lang.i64_or(self.value, rhs.value))
                }
            }
            func test() -> Bits {
                let a = Bits(value: 0b1100);
                let b = Bits(value: 0b1010);
                a | b
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn bitwise_xor_operator_protocol() {
        Test::new(
            r#"module Test
            struct Bits: Prelude.BitwiseXorOperatorProtocol {
                var value: lang.i64

                func bitwiseXor(rhs: Bits) -> Bits {
                    Bits(value: lang.i64_xor(self.value, rhs.value))
                }
            }
            func test() -> Bits {
                let a = Bits(value: 0b1100);
                let b = Bits(value: 0b1010);
                a ^ b
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn shift_left_operator_protocol() {
        Test::new(
            r#"module Test
            struct Bits: Prelude.ShiftLeftOperatorProtocol {
                var value: lang.i64

                func shiftLeft(rhs: Bits) -> Bits {
                    Bits(value: lang.i64_shl(self.value, rhs.value))
                }
            }
            func test() -> Bits {
                let a = Bits(value: 1);
                let b = Bits(value: 4);
                a << b
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn shift_right_operator_protocol() {
        Test::new(
            r#"module Test
            struct Bits: Prelude.ShiftRightOperatorProtocol {
                var value: lang.i64

                func shiftRight(rhs: Bits) -> Bits {
                    Bits(value: lang.i64_signed_shr(self.value, rhs.value))
                }
            }
            func test() -> Bits {
                let a = Bits(value: 16);
                let b = Bits(value: 2);
                a >> b
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod unary_protocols {
    use super::*;

    #[test]
    fn negate_operator_protocol() {
        Test::new(
            r#"module Test
            struct Number: Prelude.NegateOperatorProtocol {
                var value: lang.i64

                func negate() -> Number {
                    Number(value: lang.i64_neg(self.value))
                }
            }
            func test() -> Number {
                let a = Number(value: 5);
                -a
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn bitwise_not_operator_protocol() {
        Test::new(
            r#"module Test
            struct Bits: Prelude.BitwiseNotOperatorProtocol {
                var value: lang.i64

                func bitwiseNot() -> Bits {
                    Bits(value: lang.i64_not(self.value))
                }
            }
            func test() -> Bits {
                let a = Bits(value: 0b1010);
                ~a
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn logical_not_operator_protocol() {
        Test::new(
            r#"module Test
            struct Flag: Prelude.LogicalNotOperatorProtocol {
                var value: lang.i1

                func logicalNot() -> lang.i1 {
                    lang.i1_not(self.value)
                }
            }
            func test() -> lang.i1 {
                let f = Flag(value: true);
                not f
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod multiple_operators {
    use super::*;

    #[test]
    fn type_with_multiple_operators() {
        Test::new(
            r#"module Test
            struct Number: Prelude.AddOperatorProtocol, Prelude.SubtractOperatorProtocol, Prelude.EqualsOperatorProtocol {
                var value: lang.i64

                func add(rhs: Number) -> Number {
                    Number(value: lang.i64_add(self.value, rhs.value))
                }
                func subtract(rhs: Number) -> Number {
                    Number(value: lang.i64_sub(self.value, rhs.value))
                }
                func equals(rhs: Number) -> lang.i1 {
                    lang.i64_eq(self.value, rhs.value)
                }
            }
            func test() -> lang.i1 {
                let a = Number(value: 5);
                let b = Number(value: 3);
                let c = Number(value: 2);
                (a - b) == c
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn chained_operators() {
        Test::new(
            r#"module Test
            struct Number: Prelude.AddOperatorProtocol {
                var value: lang.i64

                func add(rhs: Number) -> Number {
                    Number(value: lang.i64_add(self.value, rhs.value))
                }
            }
            func test() -> Number {
                let a = Number(value: 1);
                let b = Number(value: 2);
                let c = Number(value: 3);
                a + b + c
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod errors {
    use super::*;

    #[test]
    fn operator_without_protocol_conformance() {
        Test::new(
            r#"module Test
            struct Number {
                var value: lang.i64
            }
            func test() -> Number {
                let a = Number(value: 1);
                let b = Number(value: 2);
                a + b
            }
        "#,
        )
        .expect(HasError("add"));
    }

    #[test]
    fn mismatched_operator_types() {
        Test::new(
            r#"module Test
            struct NumberA: Prelude.AddOperatorProtocol {
                var value: lang.i64
                func add(rhs: NumberA) -> NumberA {
                    NumberA(value: lang.i64_add(self.value, rhs.value))
                }
            }
            struct NumberB {
                var value: lang.i64
            }
            func test() {
                let a = NumberA(value: 1);
                let b = NumberB(value: 2);
                a + b
            }
        "#,
        )
        .expect(Fails);
    }
}

mod with_generics {
    use super::*;

    #[test]
    fn generic_type_with_operator() {
        Test::new(
            r#"module Test
            struct Wrapper[T] where T: Prelude.AddOperatorProtocol {
                var inner: T
            }
            extend Wrapper[T]: Prelude.AddOperatorProtocol where T: Prelude.AddOperatorProtocol {
                func add(rhs: Wrapper[T]) -> Wrapper[T] {
                    Wrapper[T](inner: self.inner.add(rhs.inner))
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}
