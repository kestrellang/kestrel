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

mod protocol_conformance {
    use super::*;

    #[test]
    fn same_label_same_protocol_conformance_is_duplicate() {
        // Two inits with same signature that don't implement different protocols
        // ARE duplicates even if they have different implementations
        Test::new(
            r#"module Test
            struct Wrapper {
                let value: ()

                init(from value: ()) { self.value = value }
                init(from value: ()) { self.value = () }
            }
        "#,
        )
        .expect(HasError("duplicate initializer signature"));
    }

    #[test]
    fn different_arity_with_same_label_start_is_valid() {
        // Two inits with same first label but different arity are valid overloads
        // (arity-based overloading, not protocol-based)
        Test::new(
            r#"module Test
            struct Widget {
                let x: ()

                init(value: ()) { x = value }
                init(value: (), extra: ()) { x = value }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn two_protocols_same_method_label_different_types() {
        // Two different protocols each require a method with the same label
        // A struct implementing both can have methods with same label but different types
        Test::new(
            r#"module Test
            protocol Alpha {
                func process(value: ()) -> ()
            }

            protocol Beta {
                func process(value: ((), ())) -> ()
            }

            struct Handler: Alpha, Beta {
                func process(value: ()) -> () { () }
                func process(value: ((), ())) -> () { () }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn two_protocols_same_init_label_different_types() {
        // Two different protocols each require an init with the same label
        // A struct implementing both can have inits with same label but different types
        Test::new(
            r#"module Test
            protocol Alpha {
                init(value: ())
            }

            protocol Beta {
                init(value: ((), ()))
            }

            struct Widget: Alpha, Beta {
                let x: ()

                init(value: ()) { x = value }
                init(value: ((), ())) { x = () }
            }
        "#,
        )
        .expect(Compiles);
    }
}
