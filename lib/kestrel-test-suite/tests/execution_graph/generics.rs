//! Generic MIR tests.
//!
//! Tests for generic types and functions including:
//! - Generic functions
//! - Generic structs
//! - Generic methods
//! - Monomorphization

use kestrel_test_suite::mir::*;
use kestrel_test_suite::*;

// ============================================================================
// GENERIC FUNCTIONS
// ============================================================================

mod generic_functions {
    use super::*;

    #[test]
    fn identity_function() {
        // Based on tmp/07_generics.ks
        Test::new(
            r#"
            module Main

            func identity[T](x: T) -> T {
                x
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.identity")
                .has_type_params(1)
                .has_param("x", MirTy::type_param("T"))
                .returns(MirTy::type_param("T")),
        );
    }

    #[test]
    fn generic_function_with_multiple_type_params() {
        Test::new(
            r#"
            module Main

            func swap[A, B](a: A, b: B) -> (B, A) {
                (b, a)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.swap")
                .has_type_params(2)
                .has_param("a", MirTy::type_param("A"))
                .has_param("b", MirTy::type_param("B")),
        );
    }

    #[test]
    fn generic_function_called_with_concrete_types() {
        Test::new(
            r#"
            module Main

            func identity[T](x: T) -> T { x }

            func main() -> Int {
                identity[Int](42)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_function("Main.main").calls("Main.identity"));
    }
}

// ============================================================================
// GENERIC STRUCTS
// ============================================================================

mod generic_structs {
    use super::*;

    #[test]
    fn generic_struct_definition() {
        // Based on tmp/07_generics.ks
        Test::new(
            r#"
            module Main

            struct Pair[A, B] {
                let first: A
                let second: B
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_struct("Main.Pair")
                .has_type_params(2)
                .has_field("first", MirTy::type_param("A"))
                .has_field("second", MirTy::type_param("B")),
        );
    }

    #[test]
    fn generic_struct_with_methods() {
        Test::new(
            r#"
            module Main

            struct Box[T] {
                let value: T
                
                func get() -> T {
                    self.value
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_struct("Main.Box")
                .has_type_params(1)
                .has_field("value", MirTy::type_param("T")),
        )
        .expect(
            Mir::mir_function("Main.Box.get")
                .has_type_params(1)
                .returns(MirTy::type_param("T")),
        );
    }

    #[test]
    fn generic_struct_construction() {
        Test::new(
            r#"
            module Main

            struct Box[T] {
                let value: T
            }

            func makeBox() -> Box[Int] {
                Box[Int](value: 42)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.makeBox")
                .returns(MirTy::generic("Main.Box", vec![MirTy::I64]))
                .any_block(|b| {
                    b.has_statement(StatementPattern::Construct {
                        ty: "Main.Box[i64]".to_string(),
                    })
                }),
        );
    }
}

// ============================================================================
// GENERIC METHODS
// ============================================================================

mod generic_methods {
    use super::*;

    #[test]
    fn pair_get_first() {
        // Based on tmp/07_generics.ks
        Test::new(
            r#"
            module Main

            struct Pair[A, B] {
                let first: A
                let second: B
                
                func getFirst() -> A {
                    self.first
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.Pair.getFirst")
                .has_type_params(2)
                .returns(MirTy::type_param("A"))
                .has_param(
                    "self",
                    MirTy::ref_(MirTy::generic(
                        "Main.Pair",
                        vec![MirTy::type_param("A"), MirTy::type_param("B")],
                    )),
                ),
        );
    }

    #[test]
    fn pair_get_second() {
        Test::new(
            r#"
            module Main

            struct Pair[A, B] {
                let first: A
                let second: B
                
                func getSecond() -> B {
                    self.second
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.Pair.getSecond")
                .has_type_params(2)
                .returns(MirTy::type_param("B")),
        );
    }

    #[test]
    fn method_call_on_generic_instance() {
        Test::new(
            r#"
            module Main

            struct Box[T] {
                let value: T
                
                func get() -> T {
                    self.value
                }
            }

            func main() -> Int {
                let b = Box[Int](value: 42);
                b.get()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_function("Main.main").calls("Main.Box.get"));
    }
}

// ============================================================================
// NESTED GENERICS
// ============================================================================

mod nested_generics {
    use super::*;

    #[test]
    fn generic_struct_with_generic_field() {
        // Based on tmp/34_generic_struct_with_generic_field.ks
        Test::new(
            r#"
            module Main

            struct Inner[T] {
                let value: T
            }

            struct Outer[T] {
                let inner: Inner[T]
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_struct("Main.Inner")
                .has_type_params(1)
                .has_field("value", MirTy::type_param("T")),
        )
        .expect(Mir::mir_struct("Main.Outer").has_type_params(1).has_field(
            "inner",
            MirTy::generic("Main.Inner", vec![MirTy::type_param("T")]),
        ));
    }
}

// ============================================================================
// GENERIC ENUMS
// ============================================================================

mod generic_enums {
    use super::*;

    #[test]
    fn option_enum() {
        // Based on tmp/19_generic_enum.ks
        Test::new(
            r#"
            module Main

            enum Option[T] {
                case Some(value: T)
                case None
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_enum("Main.Option")
                .has_type_params(1)
                .has_case("Some")
                .has_case("None"),
        );
    }

    #[test]
    fn result_enum() {
        Test::new(
            r#"
            module Main

            enum Result[T, E] {
                case Ok(value: T)
                case Err(error: E)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_enum("Main.Result")
                .has_type_params(2)
                .has_case("Ok")
                .has_case("Err"),
        );
    }
}

// ============================================================================
// RECURSIVE GENERIC TYPES
// ============================================================================

mod recursive_generics {
    use super::*;

    #[test]
    fn linked_list() {
        // Based on tmp/25_generic_recursive.ks
        Test::new(
            r#"
            module Main

            indirect enum List[T] {
                case Cons(head: T, tail: List[T])
                case Nil
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_enum("Main.List")
                .has_type_params(1)
                .has_case("Cons")
                .has_case("Nil"),
        );
    }

    #[test]
    fn binary_tree() {
        Test::new(
            r#"
            module Main

            indirect enum Tree[T] {
                case Node(value: T, left: Tree[T], right: Tree[T])
                case Leaf
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_enum("Main.Tree")
                .has_type_params(1)
                .has_case("Node")
                .has_case("Leaf"),
        );
    }
}

// ============================================================================
// GENERIC CLOSURES
// ============================================================================

mod generic_closures {
    use super::*;

    #[test]
    fn generic_function_returning_closure() {
        // Based on tmp/22_generic_closure.ks
        Test::new(
            r#"
            module Main

            func apply[T, U](f: (T) -> U, x: T) -> U {
                f(x)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.apply")
                .has_type_params(2)
                .returns(MirTy::type_param("U"))
                .calls_escaping(),
        );
    }
}

// ============================================================================
// USAGE TESTS
// ============================================================================

mod usage {
    use super::*;

    #[test]
    fn main_uses_generics() {
        // Based on tmp/07_generics.ks
        Test::new(
            r#"
            module Main

            func identity[T](x: T) -> T { x }

            struct Pair[A, B] {
                let first: A
                let second: B
                
                func getFirst() -> A { self.first }
                func getSecond() -> B { self.second }
            }

            func main() -> Int {
                let x = identity[Int](42);
                let s = identity[Bool](true);
                
                let p = Pair[Int, Int](first: 10, second: 20);
                let a = p.getFirst();
                let b = p.getSecond();
                
                x + a + b
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.main")
                .returns(MirTy::I64)
                .calls("Main.identity")
                .calls("Main.Pair.getFirst")
                .calls("Main.Pair.getSecond"),
        );
    }
}
