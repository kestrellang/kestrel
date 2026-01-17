//! Tests for generics implementation
//!
//! This file tests generic type declarations, type parameters,
//! where clauses, and related validation.

use kestrel_test_suite::*;

mod basic_parsing {
    use super::*;

    #[test]
    fn generic_struct_single_param() {
        Test::new("module Test\nstruct Box[T] {}")
            .expect(Compiles)
            .expect(
                Symbol::new("Box")
                    .is(SymbolKind::Struct)
                    .has(Behavior::IsGeneric(true))
                    .has(Behavior::TypeParamCount(1)),
            );
    }

    #[test]
    fn generic_struct_multiple_params() {
        Test::new("module Test\nstruct Pair[A, B] {}")
            .expect(Compiles)
            .expect(
                Symbol::new("Pair")
                    .is(SymbolKind::Struct)
                    .has(Behavior::IsGeneric(true))
                    .has(Behavior::TypeParamCount(2)),
            );
    }

    #[test]
    fn generic_struct_three_params() {
        Test::new("module Test\nstruct Triple[X, Y, Z] {}")
            .expect(Compiles)
            .expect(
                Symbol::new("Triple")
                    .is(SymbolKind::Struct)
                    .has(Behavior::IsGeneric(true))
                    .has(Behavior::TypeParamCount(3)),
            );
    }

    #[test]
    fn non_generic_struct() {
        Test::new("module Test\nstruct Plain {}")
            .expect(Compiles)
            .expect(
                Symbol::new("Plain")
                    .is(SymbolKind::Struct)
                    .has(Behavior::IsGeneric(false))
                    .has(Behavior::TypeParamCount(0)),
            );
    }

    #[test]
    fn generic_protocol() {
        Test::new(
            r#"module Test
            protocol Container[T] {
                func get() -> ()
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Container")
                .is(SymbolKind::Protocol)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1)),
        );
    }

    #[test]
    fn generic_function() {
        Test::new("module Test\nfunc identity[T](value: T) -> T { value }")
            .expect(Compiles)
            .expect(
                Symbol::new("identity")
                    .is(SymbolKind::Function)
                    .has(Behavior::IsGeneric(true))
                    .has(Behavior::TypeParamCount(1))
                    .has(Behavior::ParameterCount(1))
                    .has(Behavior::HasBody(true)),
            );
    }

    #[test]
    fn generic_type_alias() {
        Test::new("module Test\ntype List[T] = T;")
            .expect(Compiles)
            .expect(
                Symbol::new("List")
                    .is(SymbolKind::TypeAlias)
                    .has(Behavior::IsGeneric(true))
                    .has(Behavior::TypeParamCount(1)),
            );
    }
}

mod defaults {
    use super::*;

    #[test]
    fn type_param_with_default() {
        Test::new("module Test\nstruct Map[K, V = String] {}")
            .expect(Compiles)
            .expect(
                Symbol::new("Map")
                    .is(SymbolKind::Struct)
                    .has(Behavior::IsGeneric(true))
                    .has(Behavior::TypeParamCount(2)),
            );
    }

    #[test]
    fn all_params_with_defaults() {
        Test::new("module Test\nstruct Wrapper[T = Int] {}")
            .expect(Compiles)
            .expect(
                Symbol::new("Wrapper")
                    .is(SymbolKind::Struct)
                    .has(Behavior::IsGeneric(true))
                    .has(Behavior::TypeParamCount(1)),
            );
    }

    #[test]
    fn use_default_type_argument() {
        // Even with defaults, an explicit type argument list must provide full arity.
        Test::new(
            r#"module Test
            struct Map[K, V = String] { }
            type IntMap = Map[Int];
        "#,
        )
        .expect(HasError("too few type arguments"));
    }

    #[test]
    fn override_default_type_argument() {
        // Can still provide explicit value for defaulted parameter
        Test::new(
            r#"module Test
            struct Map[K, V = String] { }
            type IntToInt = Map[Int, Int];
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Map").has(Behavior::TypeParamCount(2)))
        .expect(Symbol::new("IntToInt").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn multiple_defaults() {
        Test::new(
            r#"module Test
            struct Config[A, B = Int, C = String] { }
            type SimpleConfig = Config[Bool];
            type CustomConfig = Config[Bool, Float];
        "#,
        )
        .expect(HasError("too few type arguments"));
    }
}

mod validation {
    use super::*;

    #[test]
    fn duplicate_type_param_error() {
        Test::new("module Test\nstruct Bad[T, T] {}")
            .expect(HasError("duplicate type parameter 'T'"));
    }

    #[test]
    fn duplicate_type_param_in_function() {
        Test::new("module Test\nfunc bad[A, A]() { }")
            .expect(HasError("duplicate type parameter 'A'"));
    }

    #[test]
    fn default_ordering_error() {
        Test::new("module Test\nstruct Bad[T = Int, U] {}")
            .expect(HasError("with default must come after"));
    }

    #[test]
    fn default_ordering_valid() {
        // This should compile - defaults come after non-defaults
        Test::new("module Test\nstruct Good[T, U = Int] {}")
            .expect(Compiles)
            .expect(Symbol::new("Good").has(Behavior::TypeParamCount(2)));
    }

    #[test]
    fn shadowed_type_param_in_method() {
        // Inner function's T shadows struct's T
        Test::new(
            r#"module Test
            struct Box[T] {
                func identity[T](value: T) -> T { value }
            }"#,
        )
        .expect(HasError("shadows"));
    }

    #[test]
    fn shadowed_type_param_different_name_ok() {
        // Different names should be fine
        Test::new(
            r#"module Test
            struct Box[T] {
                func identity[U](value: U) -> U { value }
            }"#,
        )
        .expect(Compiles);
    }
}

mod where_clause {
    use super::*;

    #[test]
    fn simple_where_clause() {
        Test::new(
            r#"module Test
            protocol Equatable { }
            struct Set[T] where T: Equatable { }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Set")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1)),
        );
    }

    #[test]
    fn where_clause_multiple_bounds() {
        Test::new(
            r#"module Test
            protocol Equatable { }
            protocol Hashable { }
            struct HashSet[T] where T: Equatable and Hashable { }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("HashSet")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1)),
        );
    }

    #[test]
    fn where_clause_on_function() {
        Test::new(
            r#"module Test
            protocol Comparable { }
            func sort[T](items: T) where T: Comparable { }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("sort")
                .is(SymbolKind::Function)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn where_clause_unresolved_bound() {
        // Test that where clause with non-existent protocol produces error
        Test::new(
            r#"module Test
            struct Set[T] where T: NonExistent { }
        "#,
        )
        .expect(HasError("cannot find type 'NonExistent' in this scope"));
    }

    #[test]
    fn where_clause_bound_is_struct() {
        // Test that where clause with struct instead of protocol produces error
        Test::new(
            r#"module Test
            struct SomeStruct { }
            struct Container[T] where T: SomeStruct { }
        "#,
        )
        .expect(HasError("'SomeStruct' is not a protocol"));
    }

    #[test]
    fn where_clause_bound_is_type_alias() {
        // Test that where clause with type alias instead of protocol produces error
        Test::new(
            r#"module Test
            type MyAlias = Int;
            struct Container[T] where T: MyAlias { }
        "#,
        )
        .expect(HasError("'MyAlias' is not a protocol"));
    }

    #[test]
    fn where_clause_valid_protocol_bound() {
        // Test that valid protocol bounds work correctly
        Test::new(
            r#"module Test
            protocol Display { }
            protocol Debug { }
            struct Logger[T] where T: Display and Debug { }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Logger")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1)),
        );
    }
}

mod nested_generics {
    use super::*;

    #[test]
    fn generic_inside_generic() {
        Test::new(
            r#"module Test
            struct Outer[T] {
                struct Inner[U] { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Outer")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(
            Symbol::new("Inner")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1)),
        );
    }

    #[test]
    fn non_generic_inside_generic() {
        Test::new(
            r#"module Test
            struct Container[T] {
                struct Plain { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Container")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(
            Symbol::new("Plain")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(false))
                .has(Behavior::TypeParamCount(0)),
        );
    }
}

mod instantiation {
    use super::*;

    #[test]
    fn generic_field_type() {
        // Test that generic types can be used as field types
        Test::new(
            r#"module Test
            struct Box[T] {
                let value: T
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Box")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::FieldCount(1)),
        );
    }

    #[test]
    fn generic_return_type() {
        // Test that generic types can be used as return types
        Test::new(
            r#"module Test
            struct Box[T] {
                var value: T
            }
            func makeBox[T](v: T) -> Box[T] { Box(value: v) }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Box")
                .is(SymbolKind::Struct)
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(
            Symbol::new("makeBox")
                .is(SymbolKind::Function)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn generic_parameter_type() {
        // Test that generic types can be used as parameter types
        Test::new(
            r#"module Test
            struct Box[T] {
                var value: T
            }
            func unbox[T](box: Box[T]) -> T { box.value }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Box")
                .is(SymbolKind::Struct)
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(
            Symbol::new("unbox")
                .is(SymbolKind::Function)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn nested_generic_types() {
        // Test nested generic type usage: Box[Box[Int]]
        Test::new(
            r#"module Test
            struct Box[T] { }
            type NestedBox = Box[Box[Int]];
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Box")
                .is(SymbolKind::Struct)
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(Symbol::new("NestedBox").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn multiple_type_args() {
        // Test types with multiple type arguments
        Test::new(
            r#"module Test
            struct Map[K, V] { }
            type StringToInt = Map[String, Int];
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Map")
                .is(SymbolKind::Struct)
                .has(Behavior::TypeParamCount(2)),
        )
        .expect(Symbol::new("StringToInt").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn generic_type_in_protocol() {
        // Test generic types used in protocol method signatures
        Test::new(
            r#"module Test
            struct Box[T] { }
            protocol Container[T] {
                func wrap(value: T) -> Box[T]
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Box")
                .is(SymbolKind::Struct)
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(
            Symbol::new("Container")
                .is(SymbolKind::Protocol)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1)),
        );
    }

    #[test]
    fn tuple_with_generic() {
        // Test generic types inside tuples
        Test::new(
            r#"module Test
            struct Box[T] {
                var value: T
            }
            func pair[A, B](a: A, b: B) -> (Box[A], Box[B]) { (Box(value: a), Box(value: b)) }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Box")
                .is(SymbolKind::Struct)
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(
            Symbol::new("pair")
                .is(SymbolKind::Function)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(2))
                .has(Behavior::ParameterCount(2)),
        );
    }

    #[test]
    fn function_type_with_generic() {
        // Test generic types in function type signatures
        Test::new(
            r#"module Test
            struct Box[T] {
                var value: T
            }
            func transform[T](f: (T) -> Box[T], value: T) -> Box[T] { f(value) }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Box")
                .is(SymbolKind::Struct)
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(
            Symbol::new("transform")
                .is(SymbolKind::Function)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::ParameterCount(2)),
        );
    }
}

mod arity_errors {
    use super::*;

    #[test]
    fn too_few_type_arguments() {
        // Map[K, V] requires 2 type arguments, only 1 provided
        Test::new(
            r#"module Test
            struct Map[K, V] { }
            type Bad = Map[Int];
        "#,
        )
        .expect(HasError("too few type arguments"));
    }

    #[test]
    fn too_many_type_arguments() {
        // Box[T] takes only 1 type argument, 2 provided
        Test::new(
            r#"module Test
            struct Box[T] { }
            type Bad = Box[Int, String];
        "#,
        )
        .expect(HasError("too many type arguments"));
    }

    #[test]
    fn zero_type_arguments_when_required() {
        // Using a generic type without [] syntax is treated as an instantiation where all type
        // arguments are inferred placeholders.
        Test::new(
            r#"module Test
            struct Box[T] { }
            type Alias = Box;
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn correct_arity_with_defaults() {
        // Even with defaults, an explicit type argument list must provide full arity.
        Test::new(
            r#"module Test
            struct Map[K, V = String] { }
            type IntToInt = Map[Int, Int];
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Map")
                .is(SymbolKind::Struct)
                .has(Behavior::TypeParamCount(2)),
        )
        .expect(Symbol::new("IntToInt").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn raw_reference_infers_all_type_arguments_even_with_defaults() {
        Test::new(
            r#"module Test
            struct Map[K, V = String] { }
            type Inferred = Map;
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Inferred").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn too_few_even_with_defaults() {
        // Triple[A, B, C = Int] requires at least 2 type arguments
        Test::new(
            r#"module Test
            struct Triple[A, B, C = Int] { }
            type Bad = Triple[Int];
        "#,
        )
        .expect(HasError("too few type arguments"));
    }
}

mod non_generic_errors {
    use super::*;

    #[test]
    fn type_args_on_non_generic_struct() {
        Test::new(
            r#"module Test
            struct Plain { }
            type Bad = Plain[Int];
        "#,
        )
        .expect(HasError("does not accept type arguments"));
    }

    #[test]
    fn type_args_on_non_generic_type_alias() {
        Test::new(
            r#"module Test
            type Simple = Int;
            type Bad = Simple[String];
        "#,
        )
        .expect(HasError("does not accept type arguments"));
    }

    #[test]
    fn type_args_on_primitive() {
        Test::new(
            r#"module Test
            type Bad = Int[String];
        "#,
        )
        .expect(HasError("does not accept type arguments"));
    }
}

mod undeclared_type_params {
    use super::*;

    #[test]
    fn undeclared_in_where_clause() {
        // U is not declared in the type parameter list
        Test::new(
            r#"module Test
            protocol Equatable { }
            struct Set[T] where U: Equatable { }
        "#,
        )
        .expect(HasError("undeclared type parameter"));
    }

    #[test]
    fn undeclared_in_function_where_clause() {
        Test::new(
            r#"module Test
            protocol Comparable { }
            func sort[T](items: T) where U: Comparable { }
        "#,
        )
        .expect(HasError("undeclared type parameter"));
    }

    #[test]
    fn typo_in_where_clause() {
        // Tx is a typo for T
        Test::new(
            r#"module Test
            protocol Display { }
            struct Printer[T] where Tx: Display { }
        "#,
        )
        .expect(HasError("undeclared type parameter"));
    }
}

mod type_alias_resolution {
    use super::*;

    #[test]
    fn identity_type_alias() {
        // type Identity[T] = T should be a valid generic type alias
        Test::new(
            r#"module Test
            type Identity[T] = T;
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Identity")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1)),
        );
    }

    #[test]
    fn identity_type_alias_instantiated() {
        // Using Identity[Int] should work
        Test::new(
            r#"module Test
            type Identity[T] = T;
            type IntAlias = Identity[Int];
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Identity")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(Symbol::new("IntAlias").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn pair_type_alias() {
        // type Pair[T] = (T, T) - using type param multiple times
        Test::new(
            r#"module Test
            type Pair[T] = (T, T);
            type IntPair = Pair[Int];
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Pair")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(Symbol::new("IntPair").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn nested_type_param_in_alias() {
        // Type param used as argument to another generic
        Test::new(
            r#"module Test
            struct Box[T] { }
            type Boxed[T] = Box[T];
            type BoxedInt = Boxed[Int];
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Box")
                .is(SymbolKind::Struct)
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(
            Symbol::new("Boxed")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(Symbol::new("BoxedInt").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn type_alias_with_function_type() {
        // type Transformer[A, B] = (A) -> B
        Test::new(
            r#"module Test
            type Transformer[A, B] = (A) -> B;
            type IntToString = Transformer[Int, String];
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Transformer")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::TypeParamCount(2)),
        )
        .expect(Symbol::new("IntToString").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn generic_alias_chaining() {
        // Chain of generic type aliases
        Test::new(
            r#"module Test
            struct Box[T] { }
            type Boxed[T] = Box[T];
            type DoubleBoxed[T] = Boxed[Boxed[T]];
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Box")
                .is(SymbolKind::Struct)
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(
            Symbol::new("Boxed")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(
            Symbol::new("DoubleBoxed")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::TypeParamCount(1)),
        );
    }

    #[test]
    fn type_param_in_nested_tuple() {
        Test::new(
            r#"module Test
            type Nested[T] = ((T, Int), (String, T));
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Nested")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1)),
        );
    }
}

mod multiple_constraints {
    use super::*;

    #[test]
    fn two_params_with_separate_bounds() {
        Test::new(
            r#"module Test
            protocol Equatable { }
            protocol Hashable { }
            struct BiMap[K, V] where K: Equatable, V: Hashable { }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("BiMap")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(2)),
        );
    }

    #[test]
    fn three_params_with_mixed_bounds() {
        Test::new(
            r#"module Test
            protocol A { }
            protocol B { }
            protocol C { }
            struct Complex[X, Y, Z] where X: A, Y: B and C, Z: A { }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Complex")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(3)),
        );
    }

    #[test]
    fn same_param_multiple_separate_constraints() {
        // T has two separate constraint clauses (if syntax allows)
        Test::new(
            r#"module Test
            protocol Display { }
            protocol Debug { }
            struct Logger[T] where T: Display, T: Debug { }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Logger")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1)),
        );
    }
}

mod edge_cases {
    use super::*;

    #[test]
    fn single_letter_type_params() {
        Test::new("module Test\nstruct A[B] {}")
            .expect(Compiles)
            .expect(
                Symbol::new("A")
                    .is(SymbolKind::Struct)
                    .has(Behavior::IsGeneric(true))
                    .has(Behavior::TypeParamCount(1)),
            );
    }

    #[test]
    fn long_type_param_names() {
        Test::new("module Test\nstruct Container[ElementType, KeyType, ValueType] {}")
            .expect(Compiles)
            .expect(
                Symbol::new("Container")
                    .is(SymbolKind::Struct)
                    .has(Behavior::IsGeneric(true))
                    .has(Behavior::TypeParamCount(3)),
            );
    }

    #[test]
    fn many_type_params() {
        Test::new("module Test\nstruct Many[A, B, C, D, E, F] {}")
            .expect(Compiles)
            .expect(
                Symbol::new("Many")
                    .is(SymbolKind::Struct)
                    .has(Behavior::IsGeneric(true))
                    .has(Behavior::TypeParamCount(6)),
            );
    }

    #[test]
    fn type_param_same_name_as_struct() {
        // Type parameter named same as the struct itself
        Test::new("module Test\nstruct Box[Box] {}")
            .expect(Compiles)
            .expect(
                Symbol::new("Box")
                    .is(SymbolKind::Struct)
                    .has(Behavior::IsGeneric(true))
                    .has(Behavior::TypeParamCount(1)),
            );
    }

    #[test]
    fn self_referential_generic_error() {
        // A generic type that refers to itself creates an infinite-size type.
        // This is correctly rejected - use arrays or optional types to break the cycle.
        Test::new(
            r#"module Test
            struct Node[T] {
                let value: T
                let next: Node[T]
            }
        "#,
        )
        .expect(HasError("cannot contain itself"));
    }

    #[test]
    fn self_referential_generic_with_array_ok() {
        // Self-reference through array is allowed (array provides indirection)
        Test::new(
            r#"module Test
            struct Node[T] {
                let value: T
                let children: [Node[T]]
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Node")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::FieldCount(2)),
        );
    }

    #[test]
    fn mutually_referential_generics_error() {
        // Mutually referential structs create infinite-size types.
        // This is correctly rejected.
        Test::new(
            r#"module Test
            struct Tree[T] {
                let value: T
                let children: Forest[T]
            }
            struct Forest[T] {
                let trees: Tree[T]
            }
        "#,
        )
        .expect(HasError("circular struct containment"));
    }

    #[test]
    fn mutually_referential_generics_with_array_ok() {
        // Mutually referential structs with array indirection are allowed
        Test::new(
            r#"module Test
            struct Tree[T] {
                let value: T
                let forest: Forest[T]
            }
            struct Forest[T] {
                let trees: [Tree[T]]
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Tree")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::FieldCount(2)),
        )
        .expect(
            Symbol::new("Forest")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::FieldCount(1)),
        );
    }

    #[test]
    fn type_param_shadowing_in_nested() {
        // Inner struct has its own T that shadows outer T
        Test::new(
            r#"module Test
            struct Outer[T] {
                struct Inner[T] {
                    let value: T
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Outer")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(
            Symbol::new("Inner")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1)),
        );
    }

    #[test]
    fn generic_protocol_method_using_struct_type_param() {
        Test::new(
            r#"module Test
            struct Box[T] { }
            protocol Factory[T] {
                func create() -> Box[T]
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Box")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(
            Symbol::new("Factory")
                .is(SymbolKind::Protocol)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1)),
        );
    }

    #[test]
    fn where_clause_with_generic_bound() {
        // Bound itself is a generic type: T: Comparable[T]
        Test::new(
            r#"module Test
            protocol Comparable[U] { }
            struct Collection[T] where T: Comparable[T] { }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn deeply_nested_generics() {
        Test::new(
            r#"module Test
            struct Box[T] { }
            type Deep = Box[Box[Box[Box[Int]]]];
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Box")
                .is(SymbolKind::Struct)
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(Symbol::new("Deep").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn generic_in_optional_like_pattern() {
        Test::new(
            r#"module Test
            struct Option[T] {
                let value: T
            }
            type OptionalInt = Option[Int];
            type OptionalOptional = Option[Option[String]];
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Option")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::FieldCount(1)),
        )
        .expect(Symbol::new("OptionalInt").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("OptionalOptional").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn generic_alias_preserves_param_count() {
        Test::new(
            r#"module Test
            type Pair[A, B] = (A, B);
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Pair")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(2)),
        );
    }
}

mod constraint_enforcement {
    use super::*;

    #[test]
    fn constrained_method_call_works() {
        // Basic test: calling a protocol method on a constrained type parameter
        Test::new(
            r#"module Test
            protocol Add {
                func add(other: Self) -> Self
            }
            func addThem[T](a: T, b: T) -> T where T: Add {
                return a.add(b)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn unconstrained_type_param_method_error() {
        // Calling a method on a type parameter with no constraints should error
        Test::new(
            r#"module Test
            func bad[T](a: T, b: T) -> T {
                return a.add(b)
            }
        "#,
        )
        .expect(HasError("cannot call 'add' on type 'T'"));
    }

    #[test]
    fn method_not_in_bounds_error() {
        // Calling a method that doesn't exist in the protocol bound
        Test::new(
            r#"module Test
            protocol Add {
                func add(other: Self) -> Self
            }
            func bad[T](a: T, b: T) -> T where T: Add {
                return a.subtract(b)
            }
        "#,
        )
        .expect(HasError("no method 'subtract' found for type 'T'"));
    }

    #[test]
    fn multiple_bounds_method_call() {
        // Calling methods from multiple protocol bounds
        Test::new(
            r#"module Test
            protocol Add {
                func add(other: Self) -> Self
            }
            protocol Negate {
                func negate() -> Self
            }
            func compute[T](a: T, b: T) -> T where T: Add, T: Negate {
                let sum: T = a.add(b);
                return sum.negate()
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn self_substitution_in_return_type() {
        // The return type should be T, not Self
        Test::new(
            r#"module Test
            protocol Clone {
                func clone() -> Self
            }
            func duplicateIt[T](x: T) -> T where T: Clone {
                return x.clone()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn self_substitution_in_parameter() {
        // Parameters with Self type should accept T
        Test::new(
            r#"module Test
            protocol Combine {
                func combine(with other: Self) -> Self
            }
            func merge[T](a: T, b: T) -> T where T: Combine {
                return a.combine(with: b)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn inherited_protocol_method() {
        // Should be able to call methods from inherited protocols
        Test::new(
            r#"module Test
            protocol Equatable {
                func equals(other: Self) -> Bool
            }
            protocol Comparable: Equatable {
                func lessThan(other: Self) -> Bool
            }
            func checkEqual[T](a: T, b: T) -> Bool where T: Comparable {
                return a.equals(b)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn ambiguous_method_error() {
        // Same method signature in multiple protocols should error
        Test::new(
            r#"module Test
            protocol P1 {
                func doIt() -> Int
            }
            protocol P2 {
                func doIt() -> Int
            }
            func ambig[T](x: T) -> Int where T: P1, T: P2 {
                return x.doIt()
            }
            func main() {}
        "#,
        )
        .expect(HasError("ambiguous"));
    }

    #[test]
    fn generic_protocol_bound_not_supported() {
        // Generic protocol bounds require associated types (not yet implemented)
        Test::new(
            r#"module Test
            protocol Container[T] {
                func get() -> T
            }
            func extract[C, T](c: C) -> T where C: Container[T] {
                return c.get()
            }
            func main() {}
        "#,
        )
        .expect(HasError("generic protocol bounds are not yet supported"));
    }

    #[test]
    fn struct_method_vs_protocol_method() {
        // Calling a method on a concrete struct type should still work
        Test::new(
            r#"module Test
            struct Point {
                var x: Int
                var y: Int
                func getX() -> Int {
                    return x
                }
            }
            func usePoint(p: Point) -> Int {
                return p.getX()
            }
        "#,
        )
        .expect(Compiles);
    }

    // =========================================================================
    // Edge cases and stress tests
    // =========================================================================

    #[test]
    fn diamond_inheritance_same_method() {
        // Diamond inheritance: A <- B, A <- C, and D uses both B and C
        // Method from A should be found through either path without ambiguity
        Test::new(
            r#"module Test
            protocol A {
                func doA() -> lang.i64
            }
            protocol B: A {}
            protocol C: A {}
            func diamond[T](x: T) -> lang.i64 where T: B, T: C {
                return x.doA()
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn multiple_methods_same_protocol() {
        // Calling multiple different methods from the same protocol bound
        Test::new(
            r#"module Test
            protocol Math {
                func add(other: Self) -> Self
                func subtract(other: Self) -> Self
                func multiply(other: Self) -> Self
            }
            func compute[T](a: T, b: T, c: T) -> T where T: Math {
                let sum: T = a.add(b);
                let diff: T = sum.subtract(c);
                return diff.multiply(a)
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn chained_method_calls() {
        // Chaining method calls on constrained type parameter
        Test::new(
            r#"module Test
            protocol Chainable {
                func chain() -> Self
            }
            func chainMany[T](x: T) -> T where T: Chainable {
                return x.chain().chain().chain()
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn method_returning_different_type() {
        // Method that returns a different type (not Self)
        Test::new(
            r#"module Test
            protocol Describable {
                func describe() -> String
            }
            func getDescription[T](x: T) -> String where T: Describable {
                return x.describe()
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn method_with_multiple_parameters() {
        // Method with multiple parameters including Self
        Test::new(
            r#"module Test
            protocol Combinable {
                func combine(a: Self, b: Self) -> Self
            }
            func combineThree[T](x: T, y: T, z: T) -> T where T: Combinable {
                let partial: T = x.combine(y, z);
                return partial
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn bounds_with_and_keyword() {
        // Using 'and' to specify multiple bounds on same constraint
        Test::new(
            r#"module Test
            protocol A {
                func doA() -> Int
            }
            protocol B {
                func doB() -> Int
            }
            func both[T](x: T) -> Int where T: A and B {
                let a: Int = x.doA();
                let b: Int = x.doB();
                return a
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn ambiguous_with_and_keyword() {
        // Ambiguity when using 'and' with same method name
        Test::new(
            r#"module Test
            protocol A {
                func same() -> Int
            }
            protocol B {
                func same() -> Int
            }
            func ambig[T](x: T) -> Int where T: A and B {
                return x.same()
            }
            func main() {}
        "#,
        )
        .expect(HasError("ambiguous"));
    }

    #[test]
    fn nested_function_with_own_bounds() {
        // Nested generic function with its own type parameter
        Test::new(
            r#"module Test
            protocol Printable {
                func print() -> String
            }
            func outer[T](x: T) -> String where T: Printable {
                return x.print()
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn calling_wrong_method_with_multiple_bounds() {
        // Having multiple bounds but calling a method that exists in neither
        Test::new(
            r#"module Test
            protocol A {
                func methodA() -> Int
            }
            protocol B {
                func methodB() -> Int
            }
            func wrong[T](x: T) -> Int where T: A, T: B {
                return x.methodC()
            }
            func main() {}
        "#,
        )
        .expect(HasError("methodC"));
    }

    #[test]
    fn self_in_tuple_return() {
        // Self type in a tuple return type
        Test::new(
            r#"module Test
            protocol Pair {
                func pair() -> (Self, Self)
            }
            func getPair[T](x: T) -> (T, T) where T: Pair {
                return x.pair()
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn self_in_array_parameter() {
        // Self type in array parameter
        Test::new(
            r#"module Test
            protocol Summable {
                func sumWith(others: [Self]) -> Self
            }
            func sumAll[T](x: T, others: [T]) -> T where T: Summable {
                return x.sumWith(others)
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn three_way_ambiguity() {
        // Three protocols all have the same method - should be ambiguous
        Test::new(
            r#"module Test
            protocol A {
                func common() -> Int
            }
            protocol B {
                func common() -> Int
            }
            protocol C {
                func common() -> Int
            }
            func threeWay[T](x: T) -> Int where T: A, T: B, T: C {
                return x.common()
            }
            func main() {}
        "#,
        )
        .expect(HasError("ambiguous"));
    }

    #[test]
    fn inherited_method_not_ambiguous() {
        // Method inherited from same base should not be ambiguous
        Test::new(
            r#"module Test
            protocol Base {
                func baseMethod() -> Int
            }
            protocol Child1: Base {
                func child1Method() -> Int
            }
            protocol Child2: Base {
                func child2Method() -> Int
            }
            func useBase[T](x: T) -> Int where T: Child1, T: Child2 {
                return x.baseMethod()
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn empty_protocol_bound() {
        // Protocol with no methods - should still be valid bound
        Test::new(
            r#"module Test
            protocol Empty {}
            protocol HasMethod {
                func doIt() -> Int
            }
            func useEmpty[T](x: T) -> Int where T: Empty, T: HasMethod {
                return x.doIt()
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn type_param_passed_to_another_generic() {
        // Passing a constrained type parameter to another generic function
        Test::new(
            r#"module Test
            protocol Processable {
                func process() -> Self
            }
            func helper[T](x: T) -> T where T: Processable {
                return x.process()
            }
            func outer[U](y: U) -> U where U: Processable {
                return helper[U](y)
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn explicit_type_args_simple_function() {
        // Simple function with explicit type argument
        Test::new(
            r#"module Test
            func identity[T](x: T) -> T { return x }
            func main() {
                let y: Int = identity[Int](42);
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn explicit_type_args_multiple() {
        // Function with multiple explicit type arguments
        Test::new(
            r#"module Test
            func pair[A, B](a: A, b: B) -> A { return a }
            func main() {
                let x: Int = pair[Int, String](1, "hello");
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn explicit_type_args_with_constraint() {
        // Type argument that must satisfy a constraint
        Test::new(
            r#"module Test
            protocol Addable {
                func add(other: Self) -> Self
            }
            func double[T](x: T) -> T where T: Addable {
                return x.add(x)
            }
            func caller[U](y: U) -> U where U: Addable {
                return double[U](y)
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn explicit_type_args_nested_calls() {
        // Nested function calls with explicit type args
        Test::new(
            r#"module Test
            func wrap[T](x: T) -> T { return x }
            func main() {
                let x: Int = wrap[Int](wrap[Int](42));
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn explicit_type_args_inferred_constraint() {
        // Explicit type arg flows constraint to nested call
        Test::new(
            r#"module Test
            protocol Process {
                func run() -> Self
            }
            func inner[T](x: T) -> T where T: Process {
                return x.run()
            }
            func outer[U](y: U) -> U where U: Process {
                let result: U = inner[U](y);
                return result
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn method_call_result_used_in_expression() {
        // Using method call result in arithmetic/other expressions
        Test::new(
            r#"module Test
            protocol Valuable {
                func value() -> Int
            }
            func sumValues[T](a: T, b: T) -> Int where T: Valuable {
                let x: Int = a.value();
                let y: Int = b.value();
                return x
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn constraint_on_unused_type_param() {
        // Type parameter with constraint that's declared but never used in body
        Test::new(
            r#"module Test
            protocol Unused {
                func unused() -> Int
            }
            func ignoreConstraint[T](x: Int) -> Int where T: Unused {
                return x
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_method_with_positional_params() {
        // Protocol method with positional parameters (no external labels)
        Test::new(
            r#"module Test
            protocol Calculator {
                func calculate(left: Int, right: Int) -> Int
            }
            func doCalc[T](calc: T) -> Int where T: Calculator {
                return calc.calculate(1, 2)
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_method_with_labeled_params() {
        // Protocol method with explicit external labels
        Test::new(
            r#"module Test
            protocol Calculator {
                func calculate(left left: Int, right right: Int) -> Int
            }
            func doCalc[T](calc: T) -> Int where T: Calculator {
                return calc.calculate(left: 1, right: 2)
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn wrong_labels_on_constrained_call() {
        // Calling protocol method with wrong labels
        Test::new(
            r#"module Test
            protocol Calculator {
                func calculate(left left: Int, right right: Int) -> Int
            }
            func doCalc[T](calc: T) -> Int where T: Calculator {
                return calc.calculate(a: 1, b: 2)
            }
            func main() {}
        "#,
        )
        .expect(HasError("calculate"));
    }

    // ===== Explicit Type Argument Bug Tests =====
    // These tests document bugs found during manual testing

    #[test]
    fn explicit_type_args_too_many() {
        // Providing 2 type args to function that takes 1 should error
        Test::new(
            r#"module Test
            func identity[T](x: T) -> T { return x }
            func main() {
                let y: Int = identity[Int, String](42);
            }
        "#,
        )
        .expect(HasError("too many type arguments"));
    }

    #[test]
    fn explicit_type_args_on_non_generic() {
        // Type args on non-generic function should error
        Test::new(
            r#"module Test
            func foo(x: Int) -> Int { return x }
            func main() {
                let y: Int = foo[Int](42);
            }
        "#,
        )
        .expect(HasError("does not accept type arguments"));
    }

    #[test]
    fn explicit_type_arg_conflicts_with_inferred() {
        // identity[String](42) - 42 is Int but T=String
        Test::new(
            r#"module Test
            func identity[T](x: T) -> T { return x }
            func main() {
                let y: Int = identity[String](42);
            }
        "#,
        )
        .expect(HasError("type"));
    }

    #[test]
    fn explicit_type_arg_return_type_mismatch() {
        // identity[String] returns String, but assigned to Int
        Test::new(
            r#"module Test
            func identity[T](x: T) -> T { return x }
            func main() {
                let f = identity[String];
                let x: Int = f("hello");
            }
        "#,
        )
        .expect(HasError("type"));
    }

    #[test]
    fn generic_struct_init_wrong_field_type() {
        // Box[Int] initialized with String value
        Test::new(
            r#"module Test
            struct Box[T] {
                let value: T
            }
            func main() {
                let b: Box[Int] = Box[Int](value: "hello");
            }
        "#,
        )
        .expect(HasError("type"));
    }

    #[test]
    fn explicit_type_args_empty() {
        // identity[] with empty brackets should error
        Test::new(
            r#"module Test
            func identity[T](x: T) -> T { return x }
            func main() {
                let y: Int = identity[](42);
            }
        "#,
        )
        .expect(HasError("too few type arguments"));
    }

    #[test]
    fn explicit_type_arg_undefined_type() {
        // Using undefined type DoesNotExist in type args
        Test::new(
            r#"module Test
            func identity[T](x: T) -> T { return x }
            func main() {
                let y: Int = identity[DoesNotExist](42);
            }
        "#,
        )
        .expect(HasError("cannot find type"));
    }

    #[test]
    fn explicit_type_args_on_variable() {
        // x[Int] where x is just an Int variable
        Test::new(
            r#"module Test
            func main() {
                let x: Int = 42;
                let y = x[Int];
            }
        "#,
        )
        .expect(HasError("type"));
    }

    #[test]
    fn explicit_type_arg_not_substituted() {
        // identity[Int](1) + identity[Int](2) should work, but types show as T
        Test::new(
            r#"module Test
            func identity[T](x: T) -> T { return x }
            func main() {
                let x: Int = identity[Int](1) + identity[Int](2);
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn explicit_type_arg_tuple_type() {
        // Tuple type in type argument position
        Test::new(
            r#"module Test
            func identity[T](x: T) -> T { return x }
            func main() {
                let x: (Int, Int) = identity[(Int, Int)]((1, 2));
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn explicit_type_arg_function_type() {
        // Function type in type argument position (without lambda - lambdas not yet supported)
        Test::new(
            r#"module Test
            func identity[T](x: T) -> T { return x }
            func myFunc(x: Int) -> Int { return x }
            func main() {
                let f: (Int) -> Int = identity[(Int) -> Int](myFunc);
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn explicit_type_arg_return_type_assigned_wrong() {
        // wrap[Int] returns Box[Int] but assigned to Box[String]
        Test::new(
            r#"module Test
            struct Box[T] {
                let value: T
            }
            func wrap[T](x: T) -> Box[T] { return Box[T](value: x) }
            func main() {
                let b: Box[String] = wrap[Int](42);
            }
        "#,
        )
        .expect(HasError("type"));
    }

    #[test]
    fn explicit_type_arg_array_type() {
        // Array type in type argument position
        Test::new(
            r#"module Test
            func identity[T](x: T) -> T { return x }
            func main() {
                let arr: [Int] = identity[[Int]]([1, 2, 3]);
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn explicit_type_args_too_few() {
        // Providing 1 type arg to function that takes 2 should error
        Test::new(
            r#"module Test
            func pair[A, B](a: A, b: B) -> A { return a }
            func main() {
                let x: Int = pair[Int](1, "hello");
            }
        "#,
        )
        .expect(HasError("too few type arguments"));
    }

    #[test]
    fn explicit_type_arg_on_protocol_method() {
        // Type argument on a method from protocol constraint
        Test::new(
            r#"module Test
            protocol Mapper {
                func map[U](x: Int) -> U
            }
            func apply[T](x: T) -> Int where T: Mapper {
                return x.map[Int](1)
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }
}
