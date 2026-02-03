//! Tests for initializer where clauses
//!
//! Initializers can have their own type parameters with where clause constraints,
//! separate from the struct's type parameters.
//!
//! Syntax:
//! ```kestrel
//! init[T](params) where T: Protocol { ... }
//! ```

use kestrel_test_suite::*;

mod basic {
    use super::*;

    #[test]
    fn init_with_type_parameter() {
        Test::new(
            r#"module Test
            struct Container {
                var count: lang.i64

                init[T](items: [T]) {
                    self.count = 0
                }
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn init_with_where_clause() {
        Test::new(
            r#"module Test
            protocol Countable {
                func count() -> lang.i64
            }
            struct Wrapper {
                var total: lang.i64

                init[T](item: T) where T: Countable {
                    self.total = item.count()
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn init_with_multiple_type_params() {
        Test::new(
            r#"module Test
            struct Pair {
                var first: lang.i64
                var second: lang.i64

                init[A, B](a: A, b: B) {
                    self.first = 0;
                    self.second = 0
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod with_constraints {
    use super::*;

    #[test]
    fn init_calls_constraint_method() {
        Test::new(
            r#"module Test
            protocol Hashable {
                func hash() -> lang.i64
            }
            struct HashedContainer {
                var hashValue: lang.i64

                init[T](value: T) where T: Hashable {
                    self.hashValue = value.hash()
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn init_with_multiple_constraints() {
        Test::new(
            r#"module Test
            protocol Hashable {
                func hash() -> lang.i64
            }
            protocol Comparable {
                func compare(other: Self) -> lang.i64
            }
            struct Storage {
                var value: lang.i64

                init[T](item: T) where T: Hashable, T: Comparable {
                    self.value = item.hash()
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn init_with_associated_type_constraint() {
        Test::new(
            r#"module Test
            protocol Container {
                type Element
                func first() -> Element
            }
            struct Processor {
                var count: lang.i64

                init[C](container: C) where C: Container, C.Element = lang.i64 {
                    self.count = container.first()
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod generic_struct_with_init_params {
    use super::*;

    #[test]
    fn generic_struct_init_with_additional_type_param() {
        Test::new(
            r#"module Test
            protocol Converter[To] {
                func convert() -> To
            }
            struct Box[T] {
                var value: T

                init[From](from: From) where From: Converter[T] {
                    self.value = from.convert()
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn init_type_param_distinct_from_struct_type_param() {
        Test::new(
            r#"module Test
            protocol Factory[P] {
                func produce() -> P
            }
            struct Container[T] {
                var item: T

                init[F](factory: F) where F: Factory[T] {
                    self.item = factory.produce()
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod multiple_initializers {
    use super::*;

    #[test]
    fn multiple_inits_with_different_constraints() {
        Test::new(
            r#"module Test
            protocol Readable {
                func read() -> lang.i64
            }
            protocol Writable {
                func write(value: lang.i64)
            }
            struct Store {
                var data: lang.i64

                init[R](reader reader: R) where R: Readable {
                    self.data = reader.read()
                }

                init(value value: lang.i64) {
                    self.data = value
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn init_overloading_with_type_params() {
        Test::new(
            r#"module Test
            struct Wrapper {
                var value: lang.i64

                init[T](items items: [T]) {
                    self.value = 0
                }

                init[K, V](pairs pairs: [(K, V)]) {
                    self.value = 0
                }
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

mod usage {
    use super::*;

    #[test]
    fn call_init_with_type_param() {
        Test::new(
            r#"module Test
            protocol Measurable {
                func measure() -> lang.i64
            }
            struct Metric: Measurable {
                func measure() -> lang.i64 { 42 }
            }
            struct Result {
                var measurement: lang.i64

                init[T](source: T) where T: Measurable {
                    self.measurement = source.measure()
                }
            }
            func test() -> Result {
                let m = Metric();
                Result(m)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_type_param_from_argument() {
        Test::new(
            r#"module Test
            protocol Countable {
                func count() -> lang.i64
            }
            struct List: Countable {
                var size: lang.i64
                func count() -> lang.i64 { self.size }
            }
            struct Counter {
                var total: lang.i64

                init[T](items: T) where T: Countable {
                    self.total = items.count()
                }
            }
            func test() -> Counter {
                let list = List(size: 5);
                Counter(list)
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod errors {
    use super::*;

    #[test]
    fn constraint_not_satisfied() {
        Test::new(
            r#"module Test
            protocol Hashable {
                func hash() -> lang.i64
            }
            struct NotHashable {
                var value: lang.i64
            }
            struct Container {
                var hash: lang.i64

                init[T](item: T) where T: Hashable {
                    self.hash = item.hash()
                }
            }
            func test() -> Container {
                let n = NotHashable(value: 42);
                Container(n)
            }
        "#,
        )
        .expect(HasError("Hashable"));
    }

    #[test]
    fn type_param_not_used_in_body() {
        // This should compile - unused type params are allowed
        Test::new(
            r#"module Test
            struct Box {
                var value: lang.i64

                init[T](value: lang.i64) {
                    self.value = value
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod visibility {
    use super::*;

    #[test]
    fn public_init_with_type_param() {
        Test::new(
            r#"module Test
            protocol Convertible {
                func convert() -> lang.i64
            }
            struct Public {
                var data: lang.i64

                public init[T](from: T) where T: Convertible {
                    self.data = from.convert()
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn private_init_with_type_param() {
        Test::new(
            r#"module Test
            protocol Source {
                func value() -> lang.i64
            }
            struct Private {
                var data: lang.i64

                private init[T](source: T) where T: Source {
                    self.data = source.value()
                }

                static func create[T](source: T) -> Private where T: Source {
                    Private(source)
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}
