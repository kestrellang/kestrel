//! Tests for literal protocols
//!
//! Literal protocols allow custom types to be initialized from literal syntax.
//! When a literal is used, the compiler looks for a conformance to the appropriate
//! ExpressibleBy* protocol and calls the initializer.
//!
//! Protocols:
//! - ExpressibleByIntegerLiteral: `init(intLiteral: lang.i64)`
//! - ExpressibleByFloatLiteral: `init(floatLiteral: lang.f64)`
//! - ExpressibleByStringLiteral: `init(stringLiteral: lang.str)`
//! - ExpressibleByBoolLiteral: `init(boolLiteral: lang.i1)`
//! - ExpressibleByNilLiteral: `init(nilLiteral: ())`
//! - ExpressibleByArrayLiteral: `init(_arrayLiteralPointer:_arrayLiteralCount:)`
//! - ExpressibleByDictionaryLiteral: similar pattern

use kestrel_test_suite::*;

mod expressible_by_integer_literal {
    use super::*;

    #[test]
    fn protocol_definition() {
        Test::new(
            r#"module Test
            @builtin(.ExpressibleByIntLiteral)
            protocol ExpressibleByIntegerLiteral {
                init(intLiteral value: lang.i64)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    #[test]
    fn custom_type_from_integer_literal() {
        Test::new(
            r#"module Test
            struct MyInt: Prelude.ExpressibleByIntegerLiteral {
                var value: lang.i64

                init(intLiteral value: lang.i64) {
                    self.value = value
                }
            }
            func test() -> MyInt {
                42
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn wrapper_type_from_integer_literal() {
        Test::new(
            r#"module Test
            struct Percentage: Prelude.ExpressibleByIntegerLiteral {
                var value: lang.i64

                init(intLiteral value: lang.i64) {
                    self.value = value
                }

                func asDecimal() -> lang.f64 {
                    lang.f64_div(lang.cast_i64_f64(self.value), 100.0)
                }
            }
            func test() -> Percentage {
                let p: Percentage = 50;
                p
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod expressible_by_float_literal {
    use super::*;

    #[test]
    fn protocol_definition() {
        Test::new(
            r#"module Test
            @builtin(.ExpressibleByFloatLiteral)
            protocol ExpressibleByFloatLiteral {
                init(floatLiteral value: lang.f64)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    #[test]
    fn custom_type_from_float_literal() {
        Test::new(
            r#"module Test
            struct Temperature: Prelude.ExpressibleByFloatLiteral {
                var celsius: lang.f64

                init(floatLiteral value: lang.f64) {
                    self.celsius = value
                }
            }
            func test() -> Temperature {
                36.6
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn wrapper_for_scientific_notation() {
        Test::new(
            r#"module Test
            struct Distance: Prelude.ExpressibleByFloatLiteral {
                var meters: lang.f64

                init(floatLiteral value: lang.f64) {
                    self.meters = value
                }
            }
            func test() -> Distance {
                1.5e3
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod expressible_by_string_literal {
    use super::*;

    #[test]
    fn protocol_definition() {
        Test::new(
            r#"module Test
            @builtin(.ExpressibleByStringLiteral)
            protocol ExpressibleByStringLiteral {
                init(stringLiteral value: lang.str)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    #[test]
    fn custom_type_from_string_literal() {
        Test::new(
            r#"module Test
            struct Name: Prelude.ExpressibleByStringLiteral {
                var value: lang.str

                init(stringLiteral value: lang.str) {
                    self.value = value
                }
            }
            func test() -> Name {
                "Alice"
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn url_type_from_string_literal() {
        Test::new(
            r#"module Test
            struct URL: Prelude.ExpressibleByStringLiteral {
                var path: lang.str

                init(stringLiteral value: lang.str) {
                    self.path = value
                }
            }
            func fetch(url: URL) { }
            func test() {
                let url: URL = "https://example.com";
                fetch(url)
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod expressible_by_bool_literal {
    use super::*;

    #[test]
    fn protocol_definition() {
        Test::new(
            r#"module Test
            @builtin(.ExpressibleByBoolLiteral)
            protocol ExpressibleByBoolLiteral {
                init(boolLiteral value: lang.i1)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    #[test]
    fn custom_type_from_bool_literal() {
        Test::new(
            r#"module Test
            struct Flag: Prelude.ExpressibleByBoolLiteral {
                var enabled: lang.i1

                init(boolLiteral value: lang.i1) {
                    self.enabled = value
                }
            }
            func test() -> Flag {
                true
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn tristate_from_bool() {
        Test::new(
            r#"module Test
            // -1 = unknown, 0 = false, 1 = true
            struct Tristate: Prelude.ExpressibleByBoolLiteral {
                var state: lang.i64

                init(boolLiteral value: lang.i1) {
                    if value {
                        self.state = 1
                    } else {
                        self.state = 0
                    }
                }
            }
            func test() {
                let yes: Tristate = true;
                let no: Tristate = false;
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod expressible_by_nil_literal {
    use super::*;

    #[test]
    fn protocol_definition() {
        Test::new(
            r#"module Test
            @builtin(.ExpressibleByNilLiteral)
            protocol ExpressibleByNilLiteral {
                init(nilLiteral value: ())
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    #[test]
    fn optional_from_nil() {
        Test::new(
            r#"module Test
            enum Optional[T]: Prelude.ExpressibleByNilLiteral {
                case Some(T)
                case None

                init(nilLiteral value: ()) {
                    self = Optional.None
                }
            }
            func test() -> Optional[lang.i64] {
                nil
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod expressible_by_array_literal {
    use super::*;

    #[test]
    fn array_literal_creates_array() {
        Test::new(
            r#"module Test
            func test() -> [lang.i64] {
                [1, 2, 3]
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn empty_array_literal() {
        Test::new(
            r#"module Test
            func test() -> [lang.i64] {
                []
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn nested_array_literal() {
        Test::new(
            r#"module Test
            func test() -> [[lang.i64]] {
                [[1, 2], [3, 4], [5, 6]]
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod multiple_protocols {
    use super::*;

    #[test]
    fn type_with_multiple_literal_conformances() {
        Test::new(
            r#"module Test
            struct Number: Prelude.ExpressibleByIntegerLiteral, Prelude.ExpressibleByFloatLiteral {
                var value: lang.f64

                init(intLiteral value: lang.i64) {
                    self.value = lang.cast_i64_f64(value)
                }

                init(floatLiteral value: lang.f64) {
                    self.value = value
                }
            }
            func test() {
                let a: Number = 42;
                let b: Number = 3.14;
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod type_inference {
    use super::*;

    #[test]
    fn literal_type_inferred_from_context() {
        Test::new(
            r#"module Test
            struct Counter: Prelude.ExpressibleByIntegerLiteral {
                var count: lang.i64

                init(intLiteral value: lang.i64) {
                    self.count = value
                }
            }
            func increment(c: Counter) -> Counter {
                Counter(count: lang.i64_add(c.count, 1))
            }
            func test() -> Counter {
                increment(0)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn literal_in_generic_context() {
        Test::new(
            r#"module Test
            struct Wrapper[T] {
                var value: T
            }
            func wrap[T](value: T) -> Wrapper[T] {
                Wrapper(value: value)
            }
            func test() -> Wrapper[lang.i64] {
                wrap(42)
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod errors {
    use super::*;

    #[test]
    fn literal_without_conformance() {
        Test::new(
            r#"module Test
            struct MyType {
                var value: lang.i64
            }
            func test() -> MyType {
                42
            }
        "#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn wrong_literal_type() {
        Test::new(
            r#"module Test
            struct Name: Prelude.ExpressibleByStringLiteral {
                var value: lang.str

                init(stringLiteral value: lang.str) {
                    self.value = value
                }
            }
            func test() -> Name {
                42
            }
        "#,
        )
        .expect(HasError("type mismatch"));
    }
}

mod builtin_annotations {
    use super::*;

    #[test]
    fn builtin_default_literal_type() {
        // The @builtin annotation marks a protocol as the default for a literal type
        Test::new(
            r#"module Test
            @builtin(.ExpressibleByIntLiteral)
            protocol ExpressibleByIntegerLiteral {
                init(intLiteral value: lang.i64)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(Symbol::new("ExpressibleByIntegerLiteral")
            .is(SymbolKind::Protocol)
            .has(Behavior::HasAttribute("builtin")));
    }
}
