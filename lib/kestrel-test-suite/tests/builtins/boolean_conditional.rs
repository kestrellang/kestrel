//! Tests for the BooleanConditional protocol
//!
//! The BooleanConditional protocol allows custom types to act as boolean conditions
//! in if statements, while loops, and other conditional contexts.
//!
//! Protocol:
//! ```kestrel
//! protocol BooleanConditional {
//!     func asBool() -> Bool
//! }
//! ```
//!
//! When a type conforms to BooleanConditional, it can be used directly in:
//! - `if condition { ... }`
//! - `while condition { ... }`
//! - `guard condition else { ... }`

use kestrel_test_suite::*;

mod protocol_definition {
    use super::*;

    #[test]
    fn boolean_conditional_protocol() {
        Test::new(
            r#"module Test
            // BooleanConditional is defined in Prelude
            func test() {
                // Just verify it exists
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn type_conforms_to_boolean_conditional() {
        Test::new(
            r#"module Test
            struct Flag: Prelude.BooleanConditional {
                var enabled: lang.i1

                func asBool() -> lang.i1 {
                    self.enabled
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod in_if_statements {
    use super::*;

    #[test]
    fn custom_type_in_if_condition() {
        Test::new(
            r#"module Test
            struct NonEmpty: Prelude.BooleanConditional {
                var count: lang.i64

                func asBool() -> lang.i1 {
                    lang.i64_signed_gt(self.count, 0)
                }
            }
            func test(items: NonEmpty) -> lang.i64 {
                if items {
                    1
                } else {
                    0
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn optional_in_if_condition() {
        Test::new(
            r#"module Test
            enum Option[T]: Prelude.BooleanConditional {
                case Some(T)
                case None

                func asBool() -> lang.i1 {
                    match self {
                        .Some(_) => true,
                        .None => false
                    }
                }
            }
            func test(opt: Option[lang.i64]) -> lang.i64 {
                if opt {
                    1
                } else {
                    0
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod in_while_loops {
    use super::*;

    #[test]
    fn custom_type_in_while_condition() {
        Test::new(
            r#"module Test
            struct Counter: Prelude.BooleanConditional {
                var remaining: lang.i64

                func asBool() -> lang.i1 {
                    lang.i64_signed_gt(self.remaining, 0)
                }

                mutating func decrement() {
                    self.remaining = lang.i64_sub(self.remaining, 1)
                }
            }
            func countdown() {
                var c = Counter(remaining: 5);
                while c {
                    c.decrement();
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod truthiness {
    use super::*;

    #[test]
    fn non_zero_is_truthy() {
        Test::new(
            r#"module Test
            struct Number: Prelude.BooleanConditional {
                var value: lang.i64

                func asBool() -> lang.i1 {
                    lang.i64_ne(self.value, 0)
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn non_empty_is_truthy() {
        Test::new(
            r#"module Test
            struct Text: Prelude.BooleanConditional {
                var length: lang.i64

                func asBool() -> lang.i1 {
                    lang.i64_signed_gt(self.length, 0)
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod with_result {
    use super::*;

    #[test]
    fn result_as_boolean() {
        Test::new(
            r#"module Test
            enum Result[T, E]: Prelude.BooleanConditional {
                case Ok(T)
                case Err(E)

                func asBool() -> lang.i1 {
                    match self {
                        .Ok(_) => true,
                        .Err(_) => false
                    }
                }
            }
            func test(r: Result[lang.i64, lang.str]) -> lang.i64 {
                if r {
                    1
                } else {
                    0
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod with_generics {
    use super::*;

    #[test]
    fn generic_boolean_conditional() {
        Test::new(
            r#"module Test
            struct Box[T] {
                var hasValue: lang.i1
                var value: T
            }
            extend Box[T]: Prelude.BooleanConditional {
                func asBool() -> lang.i1 {
                    self.hasValue
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod logical_operators {
    use super::*;

    #[test]
    fn boolean_conditional_with_and() {
        Test::new(
            r#"module Test
            struct Flag: Prelude.BooleanConditional {
                var value: lang.i1
                func asBool() -> lang.i1 { self.value }
            }
            func test(a: Flag, b: Flag) -> lang.i64 {
                if lang.i1_and(a.asBool(), b.asBool()) {
                    1
                } else {
                    0
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn boolean_conditional_with_or() {
        Test::new(
            r#"module Test
            struct Flag: Prelude.BooleanConditional {
                var value: lang.i1
                func asBool() -> lang.i1 { self.value }
            }
            func test(a: Flag, b: Flag) -> lang.i64 {
                if lang.i1_or(a.asBool(), b.asBool()) {
                    1
                } else {
                    0
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod extensions {
    use super::*;

    #[test]
    fn add_boolean_conditional_via_extension() {
        Test::new(
            r#"module Test
            struct Status {
                var code: lang.i64
            }
            extend Status: Prelude.BooleanConditional {
                func asBool() -> lang.i1 {
                    lang.i64_eq(self.code, 0)
                }
            }
            func test(s: Status) -> lang.i64 {
                if s {
                    1
                } else {
                    0
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod errors {
    use super::*;

    #[test]
    fn non_boolean_conditional_in_if() {
        Test::new(
            r#"module Test
            struct NotConditional {
                var value: lang.i64
            }
            func test(n: NotConditional) -> lang.i64 {
                if n {
                    1
                } else {
                    0
                }
            }
        "#,
        )
        .expect(HasError("condition"));
    }
}
