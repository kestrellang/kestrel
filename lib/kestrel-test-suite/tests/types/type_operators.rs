//! Tests for type operators
//!
//! Type operators are syntactic sugar for common generic type patterns:
//! - `T?` → `Optional[T]`
//! - `[T]` → `Array[T]`
//! - `[K: V]` → `Dictionary[K, V]`
//! - `T throws E` → `Result[T, E]`
//!
//! Type alias normalization is implemented for basic cases. The remaining
//! ignored tests are blocked on:
//! - Array operators still using built-in TyKind::Array instead of ArrayTypeOperator
//! - Nested type operators (e.g., `Int64??`) requiring parser/syntax changes
//! - Complex compositions involving arrays
//!
//! See: docs/plans/type-operators/type-operators-plan.md for details.

use kestrel_test_suite::*;

mod optional_operator {
    use super::*;

    #[test]
    fn optional_type_basic() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            func main() -> lang.i64 {
                let some: std.num.Int64? = .Some(42);
                let _ = println(some.unwrap());
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("42\n"));
    }

    #[test]
    fn optional_none() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            func main() -> lang.i64 {
                let none: std.num.Int64? = .None;
                let _ = println(none.unwrapOr(99));
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("99\n"));
    }

    #[test]
    #[ignore = "blocked on type alias normalization"]
    fn nested_optional() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            func main() -> lang.i64 {
                let nested: std.num.Int64?? = .Some(.Some(42));
                let _ = println(nested.unwrap().unwrap());
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("42\n"));
    }

    #[test]
    fn optional_interchangeable_with_explicit() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            func takeExplicit(x: std.result.Optional[std.num.Int64]) -> std.num.Int64 {
                x.unwrapOr(0)
            }

            func main() -> lang.i64 {
                let val: std.num.Int64? = .Some(42);
                let _ = println(takeExplicit(val));
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("42\n"));
    }
}

mod array_operator {
    use super::*;

    #[test]
    #[ignore = "blocked on type alias normalization"]
    fn array_type_basic() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            func main() -> lang.i64 {
                var arr: [std.num.Int64] = std.collections.Array[std.num.Int64]();
                arr.append(10);
                arr.append(20);
                arr.append(30);
                let _ = println(arr.count());
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("3\n"));
    }

    #[test]
    #[ignore = "blocked on type alias normalization"]
    fn array_first_last() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            func main() -> lang.i64 {
                var arr: [std.num.Int64] = std.collections.Array[std.num.Int64]();
                arr.append(10);
                arr.append(20);
                arr.append(30);
                let _ = println(arr.first().unwrap());
                let _ = println(arr.last().unwrap());
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("10\n30\n"));
    }

    #[test]
    #[ignore = "blocked on type alias normalization"]
    fn nested_array() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            func main() -> lang.i64 {
                var outer: [[std.num.Int64]] = std.collections.Array[std.collections.Array[std.num.Int64]]();
                var inner: [std.num.Int64] = std.collections.Array[std.num.Int64]();
                inner.append(1);
                inner.append(2);
                outer.append(inner);
                let _ = println(outer.count());
                let _ = println(outer.first().unwrap().count());
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("1\n2\n"));
    }

    #[test]
    #[ignore = "blocked on type alias normalization"]
    fn array_interchangeable_with_explicit() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            func countExplicit(arr: std.collections.Array[std.num.Int64]) -> std.num.Int64 {
                arr.count()
            }

            func main() -> lang.i64 {
                var arr: [std.num.Int64] = std.collections.Array[std.num.Int64]();
                arr.append(1);
                arr.append(2);
                let _ = println(countExplicit(arr));
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("2\n"));
    }

    #[test]
    #[ignore = "blocked on type alias normalization"]
    fn optional_array() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            func main() -> lang.i64 {
                let some: [std.num.Int64]? = .Some(std.collections.Array[std.num.Int64]());
                let none: [std.num.Int64]? = .None;
                let _ = println(some.isSome());
                let _ = println(none.isNone());
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("true\nfalse\n"));
    }
}

mod dictionary_operator {
    use super::*;

    #[test]
    fn dictionary_type_basic() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            func main() -> lang.i64 {
                var dict: [std.num.Int64: std.num.Int64] = std.collections.Dictionary[std.num.Int64, std.num.Int64](0, 0);
                let _ = dict.insert(1, 100);
                let _ = dict.insert(2, 200);
                let _ = println(dict.count());
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("2\n"));
    }

    #[test]
    fn dictionary_get_value() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            func main() -> lang.i64 {
                var dict: [std.num.Int64: std.num.Int64] = std.collections.Dictionary[std.num.Int64, std.num.Int64](0, 0);
                let _ = dict.insert(42, 123);
                let _ = println(dict.getValue(42).unwrap());
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("123\n"));
    }

    #[test]
    fn dictionary_interchangeable_with_explicit() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            func countExplicit(dict: std.collections.Dictionary[std.num.Int64, std.num.Int64]) -> std.num.Int64 {
                dict.count()
            }

            func main() -> lang.i64 {
                var dict: [std.num.Int64: std.num.Int64] = std.collections.Dictionary[std.num.Int64, std.num.Int64](0, 0);
                let _ = dict.insert(1, 1);
                let _ = println(countExplicit(dict));
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("1\n"));
    }

    #[test]
    #[ignore = "blocked on type alias normalization"]
    fn optional_dictionary() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            func main() -> lang.i64 {
                let some: [std.num.Int64: std.num.Int64]? = .Some(std.collections.Dictionary[std.num.Int64, std.num.Int64](0, 0));
                let none: [std.num.Int64: std.num.Int64]? = .None;
                let _ = println(some.isSome());
                let _ = println(none.isNone());
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("true\nfalse\n"));
    }
}

mod result_operator {
    use super::*;

    #[test]
    fn result_type_basic() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            struct MyError {}

            func main() -> lang.i64 {
                let ok: std.num.Int64 throws MyError = .Ok(42);
                let _ = println(ok.unwrap());
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("42\n"));
    }

    #[test]
    fn result_err_case() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            struct MyError {}

            func main() -> lang.i64 {
                let err: std.num.Int64 throws MyError = .Err(MyError());
                let _ = println(err.isErr());
                let _ = println(err.unwrapOr(99));
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("true\n99\n"));
    }

    #[test]
    fn result_as_function_return() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            struct ParseError {}

            func parse(valid: std.core.Bool) -> std.num.Int64 throws ParseError {
                if valid {
                    .Ok(42)
                } else {
                    .Err(ParseError())
                }
            }

            func main() -> lang.i64 {
                let ok = parse(true);
                let err = parse(false);
                let _ = println(ok.unwrap());
                let _ = println(err.isErr());
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("42\ntrue\n"));
    }

    #[test]
    fn result_interchangeable_with_explicit() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            struct MyError {}

            func handleExplicit(r: std.result.Result[std.num.Int64, MyError]) -> std.num.Int64 {
                r.unwrapOr(0)
            }

            func main() -> lang.i64 {
                let val: std.num.Int64 throws MyError = .Ok(42);
                let _ = println(handleExplicit(val));
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("42\n"));
    }

    #[test]
    #[ignore = "blocked on type alias normalization"]
    fn optional_result_precedence() {
        // Int throws Error? should be Optional[Result[Int, Error]]
        Test::new(
            r#"module Test
            import std.io.stdio.println

            struct MyError {}

            func main() -> lang.i64 {
                let someOk: std.num.Int64 throws MyError? = .Some(.Ok(42));
                let none: std.num.Int64 throws MyError? = .None;
                let _ = println(someOk.unwrap().unwrap());
                let _ = println(none.isNone());
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("42\ntrue\n"));
    }

    #[test]
    #[ignore = "blocked on type alias normalization"]
    fn result_with_explicit_optional_error() {
        // Int throws (Error?) should be Result[Int, Optional[Error]]
        Test::new(
            r#"module Test
            import std.io.stdio.println

            struct MyError {}

            func main() -> lang.i64 {
                let okVal: std.num.Int64 throws (MyError?) = .Ok(42);
                let errNone: std.num.Int64 throws (MyError?) = .Err(.None);
                let _ = println(okVal.unwrap());
                let _ = println(errNone.isErr());
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("42\ntrue\n"));
    }
}

mod complex_composition {
    use super::*;

    #[test]
    #[ignore = "blocked on type alias normalization"]
    fn array_of_optionals() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            func main() -> lang.i64 {
                var arr: [std.num.Int64?] = std.collections.Array[std.result.Optional[std.num.Int64]]();
                arr.append(.Some(1));
                arr.append(.None);
                arr.append(.Some(3));
                let _ = println(arr.count());
                let _ = println(arr.first().unwrap().unwrap());
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("3\n1\n"));
    }

    #[test]
    #[ignore = "blocked on type alias normalization"]
    fn array_of_results() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            struct MyError {}

            func main() -> lang.i64 {
                var arr: [std.num.Int64 throws MyError] = std.collections.Array[std.result.Result[std.num.Int64, MyError]]();
                arr.append(.Ok(1));
                arr.append(.Err(MyError()));
                arr.append(.Ok(3));
                let _ = println(arr.count());
                let _ = println(arr.first().unwrap().unwrap());
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("3\n1\n"));
    }

    #[test]
    #[ignore = "blocked on type alias normalization"]
    fn dictionary_with_array_value() {
        Test::new(
            r#"module Test
            import std.io.stdio.println

            func main() -> lang.i64 {
                var dict: [std.num.Int64: [std.num.Int64]] = std.collections.Dictionary[std.num.Int64, std.collections.Array[std.num.Int64]](0, std.collections.Array[std.num.Int64]());
                var arr: [std.num.Int64] = std.collections.Array[std.num.Int64]();
                arr.append(10);
                arr.append(20);
                let _ = dict.insert(1, arr);
                let _ = println(dict.getValue(1).unwrap().count());
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(StdoutEquals("2\n"));
    }
}
