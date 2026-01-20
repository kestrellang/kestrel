//! Type inference tests
//!
//! These tests verify that the type inference system correctly infers types
//! when they are not explicitly annotated.

use kestrel_test_suite::*;

mod basic_inference {
    use super::*;

    #[test]
    fn infer_int_from_literal() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    let x = 42;
    x
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_string_from_literal() {
        Test::new(
            r#"
module Main

func test() -> lang.str {
    let x = "hello";
    x
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_bool_from_literal() {
        Test::new(
            r#"
module Main

func test() -> lang.i1 {
    let x = true;
    x
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_from_another_variable() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    let x: lang.i64 = 42;
    let y = x;
    y
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_chain_of_variables() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    let a = 1;
    let b = a;
    let c = b;
    c
}
"#,
        )
        .expect(Compiles);
    }
}

mod inference_from_expressions {
    use super::*;

    #[test]
    fn infer_from_binary_op() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    let x = lang.i64_add(1, 2);
    x
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_from_comparison() {
        Test::new(
            r#"
module Main

func test() -> lang.i1 {
    let x = lang.i64_eq(1, 2);
    x
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_from_function_call() {
        Test::new(
            r#"
module Main

func getInt() -> lang.i64 { 42 }

func test() -> lang.i64 {
    let x = getInt();
    x
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_from_method_call() {
        Test::new(
            r#"
module Main

struct Foo {
    func bar() -> lang.i64 { 42 }
}

func test() -> lang.i64 {
    let f = Foo();
    let x = f.bar();
    x
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_struct_from_constructor() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func test() {
    let p = Point(x: 1, y: 2);
    let x: lang.i64 = p.x;
}
"#,
        )
        .expect(Compiles);
    }
}

mod inference_with_generics {
    use super::*;

    #[test]
    fn infer_generic_struct_type_param() {
        Test::new(
            r#"
module Main

struct Box[T] {
    var value: T
}

func test() -> lang.i64 {
    let b = Box(value: 42);
    b.value
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_generic_from_context() {
        Test::new(
            r#"
module Main

struct Box[T] {
    var value: T
}

func test() {
    let b = Box(value: 42);
    let x: lang.i64 = b.value;
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_generic_method_return() {
        Test::new(
            r#"
module Main

struct Box[T] {
    var value: T

    func read() -> T { self.value }
}

func test() -> lang.i64 {
    let b = Box(value: 42);
    b.get()
}
"#,
        )
        .expect(Compiles);
    }
}

mod inference_in_control_flow {
    use super::*;

    #[test]
    fn infer_from_if_else() {
        Test::new(
            r#"
module Main

func test(cond: lang.i1) -> lang.i64 {
    let x = if cond { 1 } else { 2 };
    x
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_in_while_body() {
        Test::new(
            r#"
module Main

func test() {
    var count = 0;
    while lang.i64_signed_lt(count, 10) {
        let doubled = lang.i64_mul(count, 2);
        count = lang.i64_add(count, 1);
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_in_loop_body() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    var sum = 0;
    var i = 0;
    loop {
        if lang.i64_signed_ge(i, 10) {
            break
        }
        let x = lang.i64_mul(i, i);
        sum = lang.i64_add(sum, x);
        i = lang.i64_add(i, 1);
    }
    sum
}
"#,
        )
        .expect(Compiles);
    }
}

mod inference_errors {
    use super::*;

    // TODO: Add test for uninitialized variables once that validation is implemented
    // fn ambiguous_inference_needs_annotation() { ... }

    #[test]
    fn inferred_type_mismatch_with_usage() {
        Test::new(
            r#"
module Main

func test() {
    let x = 42;
    let y: lang.str = x;
}
"#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn inferred_type_mismatch_in_return() {
        Test::new(
            r#"
module Main

func test() -> lang.str {
    let x = 42;
    x
}
"#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn inferred_type_mismatch_in_assignment() {
        Test::new(
            r#"
module Main

func test() {
    var x = "hello";
    x = 42
}
"#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn inferred_type_mismatch_in_function_arg() {
        Test::new(
            r#"
module Main

func takeString(s: lang.str) {}

func test() {
    let x = 42;
    takeString(x)
}
"#,
        )
        .expect(HasError("type mismatch"));
    }
}

mod tuple_inference {
    use super::*;

    #[test]
    fn infer_tuple_type() {
        Test::new(
            r#"
module Main

func test() {
    let t = (1, "hello", true);
    let x: lang.i64 = t.0;
    let y: lang.str = t.1;
    let z: lang.i1 = t.2;
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_nested_tuple() {
        Test::new(
            r#"
module Main

func test() {
    let t = ((1, 2), (3, 4));
    let inner = t.0;
    let x: lang.i64 = inner.0;
}
"#,
        )
        .expect(Compiles);
    }
}

mod array_inference {
    use super::*;

    #[test]
    fn infer_array_from_elements() {
        Test::new(
            r#"
module Main

func test() {
    let arr = [1, 2, 3];
    let x: [lang.i64] = arr;
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_array_element_type_mismatch() {
        Test::new(
            r#"
module Main

func test() {
    let arr = [1, 2, 3];
    let x: [lang.str] = arr;
}
"#,
        )
        .expect(HasError("type mismatch"));
    }
}

mod bidirectional_inference {
    use super::*;

    #[test]
    fn infer_from_expected_return_type() {
        // The return type provides context for inference
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    let x = 42;
    x
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_from_variable_annotation() {
        // The variable annotation provides context
        Test::new(
            r#"
module Main

func test() {
    let x: lang.i64 = 42;
    let y = lang.i64_add(x, 1);
    let z: lang.i64 = y;
}
"#,
        )
        .expect(Compiles);
    }
}

mod static_method_type_substitution {
    use super::*;

    #[test]
    fn static_method_in_extension_substitutes_type_param() {
        // When calling Box[lang.i64].wrap(42), T should be substituted with lang.i64
        Test::new(
            r#"
module Main

struct Box[T] {
    var value: T
}

extend Box[T] {
    static func wrap(v: T) -> Box[T] {
        Box[T](value: v)
    }
}

func test() -> Box[lang.i64] {
    let b = Box[lang.i64].wrap(42);
    b
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn static_method_in_extension_field_access() {
        Test::new(
            r#"
module Main

struct Box[T] {
    var value: T
}

extend Box[T] {
    static func wrap(v: T) -> Box[T] {
        Box[T](value: v)
    }
}

func test() -> lang.i64 {
    let b = Box[lang.i64].wrap(42);
    b.value
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn static_method_on_struct_substitutes_type_param() {
        // Static methods directly on struct should also substitute
        Test::new(
            r#"
module Main

struct Factory[T] {
    var product: T

    static func make(value: T) -> Factory[T] {
        Factory[T](product: value)
    }
}

func test() -> Factory[lang.i64] {
    Factory[lang.i64].make(42)
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn static_method_on_struct_field_access() {
        Test::new(
            r#"
module Main

struct Factory[T] {
    var product: T

    static func make(value: T) -> Factory[T] {
        Factory[T](product: value)
    }
}

func test() -> lang.i64 {
    let f = Factory[lang.i64].make(42);
    f.product
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn static_method_infers_type_from_args() {
        // When calling Box.wrap(42) without explicit type args, T should be inferred from 42
        Test::new(
            r#"
module Main

struct Box[T] {
    var value: T
}

extend Box[T] {
    static func wrap(v: T) -> Box[T] {
        Box[T](value: v)
    }
}

func test() -> lang.i64 {
    let b = Box.wrap(42);
    b.value
}
"#,
        )
        .expect(Compiles);
    }
}

mod generic_method_type_substitution {
    use super::*;

    #[test]
    fn generic_method_in_extension_infers_type_param() {
        // When calling wrapper.rewrap("hello"), U should be inferred as String
        Test::new(
            r#"
module Main

struct Wrapper[T] {
    var inner: T
}

extend Wrapper[T] {
    func rewrap[U](newValue: U) -> Wrapper[U] {
        Wrapper[U](inner: newValue)
    }
}

func test() -> Wrapper[lang.str] {
    let w = Wrapper[lang.i64](inner: 42);
    w.rewrap("hello")
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn generic_method_chained_calls() {
        Test::new(
            r#"
module Main

struct Wrapper[T] {
    var inner: T
}

extend Wrapper[T] {
    func rewrap[U](newValue: U) -> Wrapper[U] {
        Wrapper[U](inner: newValue)
    }
}

func test() -> Wrapper[lang.i1] {
    let w = Wrapper[lang.i64](inner: 42);
    let w2 = w.rewrap("hello");
    w2.rewrap(true)
}
"#,
        )
        .expect(Compiles);
    }
}
