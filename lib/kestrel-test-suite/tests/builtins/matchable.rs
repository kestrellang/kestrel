//! Tests for the Matchable protocol
//!
//! The Matchable protocol allows custom types to define their own equality semantics
//! for pattern matching. When a type conforms to Matchable, literal patterns in match
//! expressions will use the witness method call instead of direct comparison.
//!
//! Protocol:
//! ```kestrel
//! protocol Matchable {
//!     func matches(self, other: Self) -> Bool
//! }
//! ```

use kestrel_test_suite::*;

mod protocol_definition {
    use super::*;

    #[test]
    fn matchable_protocol() {
        Test::new(
            r#"module Test
            // Matchable is defined in Prelude
            func test() {
                // Just verify it exists
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn type_conforms_to_matchable() {
        Test::new(
            r#"module Test
            struct Number: Prelude.Matchable {
                var value: lang.i64

                func matches(other: Number) -> lang.i1 {
                    lang.i64_eq(self.value, other.value)
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod custom_matching {
    use super::*;

    #[test]
    fn matchable_protocol_implementation() {
        Test::new(
            r#"module Test
            struct CaseInsensitiveChar: Prelude.Matchable {
                var char: lang.i64

                func matches(other: CaseInsensitiveChar) -> lang.i1 {
                    // Simplified: just compare the values
                    lang.i64_eq(self.char, other.char)
                }
            }
            func useMatchable(a: CaseInsensitiveChar, b: CaseInsensitiveChar) -> lang.i1 {
                a.matches(b)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn custom_approximate_matching() {
        Test::new(
            r#"module Test
            struct ApproxFloat: Prelude.Matchable {
                var value: lang.f64
                var epsilon: lang.f64

                func matches(other: ApproxFloat) -> lang.i1 {
                    // Check if values are within epsilon of each other
                    let diff = lang.f64_sub(self.value, other.value);
                    let absDiff = if lang.f64_lt(diff, 0.0) {
                        lang.f64_neg(diff)
                    } else {
                        diff
                    };
                    lang.f64_le(absDiff, self.epsilon)
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod with_enums {
    use super::*;

    #[test]
    fn enum_with_matchable_payload() {
        Test::new(
            r#"module Test
            struct Version: Prelude.Matchable {
                var major: lang.i64
                var minor: lang.i64

                func matches(other: Version) -> lang.i1 {
                    // Only match on major version
                    lang.i64_eq(self.major, other.major)
                }
            }
            enum Software {
                case App(Version)
                case Library(Version)
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod literal_patterns {
    use super::*;

    #[test]
    fn boolean_pattern_matching() {
        // Boolean pattern matching uses comma-separated patterns
        Test::new(
            r#"module Test
            func describe(flag: lang.i1) -> lang.str {
                match flag {
                    true => "enabled",
                    false => "disabled"
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn boolean_with_wildcard() {
        Test::new(
            r#"module Test
            func describe(flag: lang.i1) -> lang.str {
                match flag {
                    true => "yes",
                    _ => "no"
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod fallback_behavior {
    use super::*;

    #[test]
    fn struct_destructuring_in_match() {
        // Struct patterns use destructuring syntax
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }
            func getX(p: Point) -> lang.i64 {
                match p {
                    Point(x: let xVal, y: _) => xVal
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
    fn generic_matchable() {
        Test::new(
            r#"module Test
            struct Box[T] where T: Prelude.Matchable {
                var value: T
            }
            extend Box[T]: Prelude.Matchable where T: Prelude.Matchable {
                func matches(other: Box[T]) -> lang.i1 {
                    self.value.matches(other.value)
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod edge_cases {
    use super::*;

    #[test]
    fn matchable_with_multiple_fields() {
        Test::new(
            r#"module Test
            struct Coordinate: Prelude.Matchable {
                var x: lang.i64
                var y: lang.i64
                var z: lang.i64

                func matches(other: Coordinate) -> lang.i1 {
                    // Only match on x and y, ignore z
                    lang.i1_and(
                        lang.i64_eq(self.x, other.x),
                        lang.i64_eq(self.y, other.y)
                    )
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn matchable_always_true() {
        Test::new(
            r#"module Test
            struct Wildcard: Prelude.Matchable {
                var ignored: lang.i64

                func matches(other: Wildcard) -> lang.i1 {
                    true
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}
