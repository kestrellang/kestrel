//! Tests for duplicate callable (function, initializer, subscript) detection.
//!
//! In Kestrel, overloading is label-based - two callables with the same
//! name and labels are duplicates regardless of parameter/return types.
//!
//! Note: Tests use only unit type () to avoid test harness type resolution issues.

use kestrel_test_suite::*;

mod duplicate_functions {
    use super::*;

    #[test]
    fn same_name_no_params_is_duplicate() {
        // Same name + no params = DUPLICATE
        Test::new(
            r#"module Test
            func process() { }
            func process() { }
        "#,
        )
        .expect(HasError("duplicate function signature"));
    }

    #[test]
    fn same_name_same_labels_is_duplicate() {
        // Same name + labels = DUPLICATE (regardless of types in real compiler)
        Test::new(
            r#"module Test
            func process(x: ()) { }
            func process(x: ()) { }
        "#,
        )
        .expect(HasError("duplicate function signature"));
    }

    #[test]
    fn same_name_different_explicit_labels_is_valid_overload() {
        // Different explicit labels = valid overload
        Test::new(
            r#"module Test
            func process(labelA x: ()) { }
            func process(labelB x: ()) { }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn no_params_vs_params_is_valid_overload() {
        // Different arity = valid overload
        Test::new(
            r#"module Test
            func greet() { }
            func greet(with name: ()) { }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn multiple_params_no_labels_is_duplicate() {
        // Without explicit labels, params are unlabeled -> same signature = duplicate
        Test::new(
            r#"module Test
            func add(a: (), b: ()) { }
            func add(x: (), y: ()) { }
        "#,
        )
        .expect(HasError("duplicate function signature"));
    }

    #[test]
    fn multiple_params_different_explicit_labels_is_valid_overload() {
        Test::new(
            r#"module Test
            func add(first a: (), second b: ()) { }
            func add(left x: (), right y: ()) { }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn duplicate_in_struct() {
        Test::new(
            r#"module Test
            struct Calculator {
                func compute(x: ()) { }
                func compute(x: ()) { }
            }
        "#,
        )
        .expect(HasError("duplicate function signature"));
    }

    #[test]
    fn duplicate_in_protocol() {
        Test::new(
            r#"module Test
            protocol Processor {
                func process(x: ()) -> ()
                func process(x: ()) -> ()
            }
        "#,
        )
        .expect(HasError("duplicate function signature"));
    }

    #[test]
    fn duplicate_in_enum() {
        Test::new(
            r#"module Test
            enum State {
                case active

                func describe(x: ()) { }
                func describe(x: ()) { }
            }
        "#,
        )
        .expect(HasError("duplicate function signature"));
    }

    #[test]
    fn duplicate_in_extension() {
        Test::new(
            r#"module Test
            struct Point { let x: () }

            extend Point {
                func move(x: ()) { }
                func move(x: ()) { }
            }
        "#,
        )
        .expect(HasError("duplicate function signature"));
    }
}

mod duplicate_initializers {
    use super::*;

    #[test]
    fn same_labels_is_duplicate() {
        // Initializers use implicit labels (unlike functions)
        Test::new(
            r#"module Test
            struct Point {
                let x: ()

                init(value: ()) { x = value }
                init(value: ()) { x = () }
            }
        "#,
        )
        .expect(HasError("duplicate initializer signature"));
    }

    #[test]
    fn different_labels_is_valid_overload() {
        // Initializers use implicit labels, so different param names = different labels
        Test::new(
            r#"module Test
            struct Point {
                let x: ()

                init(value: ()) { x = value }
                init(from: ()) { x = from }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod duplicate_subscripts {
    use super::*;

    #[test]
    fn same_labels_is_duplicate() {
        // Subscripts use implicit labels
        Test::new(
            r#"module Test
            struct Container {
                let items: ()

                subscript(index: ()) -> () { items }
                subscript(index: ()) -> () { items }
            }
        "#,
        )
        .expect(HasError("duplicate subscript signature"));
    }

    #[test]
    fn different_labels_is_valid_overload() {
        // Subscripts use implicit labels, so different param names = different labels
        Test::new(
            r#"module Test
            struct Container {
                let items: ()

                subscript(index: ()) -> () { items }
                subscript(at: ()) -> () { items }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod valid_overloads {
    use super::*;

    #[test]
    fn arity_overloading() {
        // Different number of parameters = different signatures
        Test::new(
            r#"module Test
            func log() { }
            func log(msg: ()) { }
            func log(msg: (), level: ()) { }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn label_overloading_in_struct_with_explicit_labels() {
        // Must use explicit labels for function overloading
        Test::new(
            r#"module Test
            struct Logger {
                func log(message msg: ()) { }
                func log(error err: ()) { }
                func log(warning warn: ()) { }
            }
        "#,
        )
        .expect(Compiles);
    }
}
