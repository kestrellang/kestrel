//! Tests for delegating initializers
//!
//! Delegating initializers use `self.init(...)` to call another initializer
//! of the same type. This allows code reuse between initializers.
//!
//! Syntax:
//! ```kestrel
//! init(x: Int) {
//!     self.init(x: x, y: 0)  // delegates to another initializer
//! }
//! ```

use kestrel_test_suite::*;

mod basic {
    use super::*;

    #[test]
    fn simple_delegation() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64

                init(x: lang.i64, y: lang.i64) {
                    self.x = x;
                    self.y = y
                }

                init(x: lang.i64) {
                    self.init(x, 0)
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn delegation_to_default() {
        Test::new(
            r#"module Test
            struct Counter {
                var count: lang.i64

                init() {
                    self.count = 0
                }

                init(startingAt value: lang.i64) {
                    self.init();
                    self.count = value
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn delegation_with_transformation() {
        Test::new(
            r#"module Test
            struct Temperature {
                var celsius: lang.i64

                init(celsius celsius: lang.i64) {
                    self.celsius = celsius
                }

                init(fahrenheit fahrenheit: lang.i64) {
                    self.init(celsius: lang.i64_signed_div(lang.i64_mul(lang.i64_sub(fahrenheit, 32), 5), 9))
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod chain {
    use super::*;

    #[test]
    fn chained_delegation() {
        Test::new(
            r#"module Test
            struct Box {
                var width: lang.i64
                var height: lang.i64
                var depth: lang.i64

                init(width width: lang.i64, height height: lang.i64, depth depth: lang.i64) {
                    self.width = width;
                    self.height = height;
                    self.depth = depth
                }

                init(side: lang.i64) {
                    self.init(width: side, height: side, depth: side)
                }

                init() {
                    self.init(1)
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
    fn generic_struct_delegation() {
        Test::new(
            r#"module Test
            struct Wrapper[T] {
                var value: T
                var label: lang.str

                init(value: T, label: lang.str) {
                    self.value = value;
                    self.label = label
                }

                init(value: T) {
                    self.init(value, "default")
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn delegation_in_constrained_generic() {
        Test::new(
            r#"module Test
            protocol Defaultable {
                static func default_() -> Self
            }
            struct Container[T] where T: Defaultable {
                var item: T

                init(item: T) {
                    self.item = item
                }

                init() {
                    self.init(T.default_())
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod fields_initialized {
    use super::*;

    #[test]
    fn all_fields_initialized_after_delegation() {
        Test::new(
            r#"module Test
            struct Person {
                var name: lang.str
                var age: lang.i64
                var email: lang.str

                init(name: lang.str, age: lang.i64, email: lang.str) {
                    self.name = name;
                    self.age = age;
                    self.email = email
                }

                init(name: lang.str) {
                    self.init(name, 0, "")
                }
            }
            func test() {
                let p = Person("Alice");
                let _n = p.name;
                let _a = p.age;
                let _e = p.email;
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn modify_after_delegation() {
        Test::new(
            r#"module Test
            struct Config {
                var debug: lang.i1
                var verbose: lang.i1

                init(debug: lang.i1, verbose: lang.i1) {
                    self.debug = debug;
                    self.verbose = verbose
                }

                init(debug: lang.i1) {
                    self.init(debug, false);
                    if debug {
                        self.verbose = true
                    }
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
    fn public_delegates_to_private() {
        Test::new(
            r#"module Test
            struct Secret {
                var data: lang.i64

                private init(data: lang.i64) {
                    self.data = data
                }

                public init() {
                    self.init(42)
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn internal_delegates_to_private() {
        Test::new(
            r#"module Test
            struct Internal {
                var value: lang.i64

                private init(value: lang.i64) {
                    self.value = value
                }

                internal init(fromInt n: lang.i64) {
                    self.init(n)
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod usage {
    use super::*;

    #[test]
    fn use_delegating_initializer() {
        Test::new(
            r#"module Test
            struct Rect {
                var width: lang.i64
                var height: lang.i64

                init(width: lang.i64, height: lang.i64) {
                    self.width = width;
                    self.height = height
                }

                init(size: lang.i64) {
                    self.init(size, size)
                }
            }
            func test() -> Rect {
                Rect(10)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn use_chained_delegating_initializer() {
        Test::new(
            r#"module Test
            struct Vector {
                var x: lang.i64
                var y: lang.i64
                var z: lang.i64

                init(x: lang.i64, y: lang.i64, z: lang.i64) {
                    self.x = x;
                    self.y = y;
                    self.z = z
                }

                init(x: lang.i64, y: lang.i64) {
                    self.init(x, y, 0)
                }

                init(x: lang.i64) {
                    self.init(x, 0)
                }

                init() {
                    self.init(0)
                }
            }
            func test() -> (Vector, Vector, Vector, Vector) {
                (Vector(1, 2, 3),
                 Vector(1, 2),
                 Vector(1),
                 Vector())
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod errors {
    use super::*;

    #[test]
    fn delegation_to_nonexistent_init() {
        Test::new(
            r#"module Test
            struct Bad {
                var value: lang.i64

                init() {
                    self.init(nonexistent: 42)
                }
            }
        "#,
        )
        .expect(HasError("no method 'init' on type 'Bad'"));
    }

    #[test]
    fn delegation_outside_init() {
        Test::new(
            r#"module Test
            struct Bad {
                var value: lang.i64

                init(value: lang.i64) {
                    self.value = value
                }

                func reset() {
                    self.init(value: 0)
                }
            }
        "#,
        )
        .expect(HasError("init"));
    }

    #[test]
    fn delegation_with_wrong_types() {
        Test::new(
            r#"module Test
            struct Bad {
                var value: lang.i64

                init(value: lang.i64) {
                    self.value = value
                }

                init(text: lang.str) {
                    self.init(value: text)
                }
            }
        "#,
        )
        .expect(HasError("type"));
    }
}

mod with_enum {
    use super::*;

    #[test]
    fn enum_delegating_initializer() {
        Test::new(
            r#"module Test
            enum Result[T, E] {
                case Ok(T)
                case Err(E)

                init(ok value: T) {
                    self = Result.Ok(value)
                }

                init(err error: E) {
                    self = Result.Err(error)
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}
