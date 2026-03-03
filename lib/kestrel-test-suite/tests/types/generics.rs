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
                func read() -> ()
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
        Test::new("module Test\nstruct Map[K, V = lang.str] {}")
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
        Test::new("module Test\nstruct Wrapper[T = lang.i64] {}")
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
        // Partial type argument lists are allowed when trailing parameters have defaults.
        Test::new(
            r#"module Test
            struct Map[K, V = lang.str] { }
            type IntMap = Map[lang.i64];
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("IntMap").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn override_default_type_argument() {
        // Can still provide explicit value for defaulted parameter
        Test::new(
            r#"module Test
            struct Map[K, V = lang.str] { }
            type IntToInt = Map[lang.i64, lang.i64];
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Map").has(Behavior::TypeParamCount(2)))
        .expect(Symbol::new("IntToInt").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn multiple_defaults() {
        // Can provide partial type arguments when trailing parameters have defaults.
        Test::new(
            r#"module Test
            struct Config[A, B = lang.i64, C = lang.str] { }
            type SimpleConfig = Config[lang.i1];
            type CustomConfig = Config[lang.i1, lang.f64];
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("SimpleConfig").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("CustomConfig").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn missing_required_type_argument() {
        // Still an error if a required parameter (without default) is missing.
        Test::new(
            r#"module Test
            struct Map[K, V] { }
            type BadMap = Map[lang.i64];
        "#,
        )
        .expect(HasError("too few type arguments"));
    }

    #[test]
    fn missing_required_with_trailing_defaults() {
        // Error if required parameters are missing, even if there are defaults for later params.
        Test::new(
            r#"module Test
            struct Config[A, B, C = lang.str] { }
            type BadConfig = Config[lang.i1];
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
        Test::new("module Test\nstruct Bad[T = lang.i64, U] {}")
            .expect(HasError("with default must come after"));
    }

    #[test]
    fn default_ordering_valid() {
        // This should compile - defaults come after non-defaults
        Test::new("module Test\nstruct Good[T, U = lang.i64] {}")
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
            type MyAlias = lang.i64;
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
        // Test nested generic type usage: Box[Box[lang.i64]]
        Test::new(
            r#"module Test
            struct Box[T] { }
            type NestedBox = Box[Box[lang.i64]];
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
            type StringToInt = Map[lang.str, lang.i64];
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
            type Bad = Map[lang.i64];
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
            type Bad = Box[lang.i64, lang.str];
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
            struct Map[K, V = lang.str] { }
            type IntToInt = Map[lang.i64, lang.i64];
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
            struct Map[K, V = lang.str] { }
            type Inferred = Map;
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Inferred").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn too_few_even_with_defaults() {
        // Triple[A, B, C = lang.i64] requires at least 2 type arguments
        Test::new(
            r#"module Test
            struct Triple[A, B, C = lang.i64] { }
            type Bad = Triple[lang.i64];
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
            type Bad = Plain[lang.i64];
        "#,
        )
        .expect(HasError("does not accept type arguments"));
    }

    #[test]
    fn type_args_on_non_generic_type_alias() {
        Test::new(
            r#"module Test
            type Simple = lang.i64;
            type Bad = Simple[lang.str];
        "#,
        )
        .expect(HasError("does not accept type arguments"));
    }

    #[test]
    fn type_args_on_primitive() {
        Test::new(
            r#"module Test
            type Bad = lang.i64[lang.str];
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
        // Using Identity[lang.i64] should work
        Test::new(
            r#"module Test
            type Identity[T] = T;
            type IntAlias = Identity[lang.i64];
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
            type IntPair = Pair[lang.i64];
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
            type BoxedInt = Boxed[lang.i64];
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
            type IntToString = Transformer[lang.i64, lang.str];
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
            type Nested[T] = ((T, lang.i64), (lang.str, T));
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
        .with_stdlib()
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
        .with_stdlib()
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
        // Inner struct has its own T that shadows outer T - this is disallowed
        Test::new(
            r#"module Test
            struct Outer[T] {
                struct Inner[T] {
                    let value: T
                }
            }
        "#,
        )
        .expect(HasError("shadows"));
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
            type Deep = Box[Box[Box[Box[lang.i64]]]];
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
            type OptionalInt = Option[lang.i64];
            type OptionalOptional = Option[Option[lang.str]];
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
                var sum: T = a.add(b);
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
                func equals(other: Self) -> lang.i1
            }
            protocol Comparable: Equatable {
                func lessThan(other: Self) -> lang.i1
            }
            func checkEqual[T](a: T, b: T) -> lang.i1 where T: Comparable {
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
                func doIt() -> lang.i64
            }
            protocol P2 {
                func doIt() -> lang.i64
            }
            func ambig[T](x: T) -> lang.i64 where T: P1, T: P2 {
                return x.doIt()
            }
            func main() {}
        "#,
        )
        .expect(HasError("ambiguous"));
    }

    #[test]
    fn generic_protocol_bound_now_supported() {
        // Generic protocol bounds are now supported
        Test::new(
            r#"module Test
            protocol Container[T] {
                func read() -> T
            }
            func extract[C, T](c: C) -> T where C: Container[T] {
                return c.read()
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_method_vs_protocol_method() {
        // Calling a method on a concrete struct type should still work
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64
                func getX() -> lang.i64 {
                    return x
                }
            }
            func usePoint(p: Point) -> lang.i64 {
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
                var sum: T = a.add(b);
                var diff: T = sum.subtract(c);
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
                func describe() -> lang.str
            }
            func getDescription[T](x: T) -> lang.str where T: Describable {
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
                func doA() -> lang.i64
            }
            protocol B {
                func doB() -> lang.i64
            }
            func both[T](x: T) -> lang.i64 where T: A and B {
                var a: lang.i64 = x.doA();
                var b: lang.i64 = x.doB();
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
                func same() -> lang.i64
            }
            protocol B {
                func same() -> lang.i64
            }
            func ambig[T](x: T) -> lang.i64 where T: A and B {
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
                func print() -> lang.str
            }
            func outer[T](x: T) -> lang.str where T: Printable {
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
                func methodA() -> lang.i64
            }
            protocol B {
                func methodB() -> lang.i64
            }
            func wrong[T](x: T) -> lang.i64 where T: A, T: B {
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
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn three_way_ambiguity() {
        // Three protocols all have the same method - should be ambiguous
        Test::new(
            r#"module Test
            protocol A {
                func common() -> lang.i64
            }
            protocol B {
                func common() -> lang.i64
            }
            protocol C {
                func common() -> lang.i64
            }
            func threeWay[T](x: T) -> lang.i64 where T: A, T: B, T: C {
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
                func baseMethod() -> lang.i64
            }
            protocol Child1: Base {
                func child1Method() -> lang.i64
            }
            protocol Child2: Base {
                func child2Method() -> lang.i64
            }
            func useBase[T](x: T) -> lang.i64 where T: Child1, T: Child2 {
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
                func doIt() -> lang.i64
            }
            func useEmpty[T](x: T) -> lang.i64 where T: Empty, T: HasMethod {
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
                var result: U = helper[U](y);
                return result
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
                let y: lang.i64 = identity[lang.i64](42);
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
                let x: lang.i64 = pair[lang.i64, lang.str](1, "hello");
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
                let x: lang.i64 = wrap[lang.i64](wrap[lang.i64](42));
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
                var result: U = inner[U](y);
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
                func value() -> lang.i64
            }
            func sumValues[T](a: T, b: T) -> lang.i64 where T: Valuable {
                var x: lang.i64 = a.value();
                var y: lang.i64 = b.value();
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
                func unused() -> lang.i64
            }
            func ignoreConstraint[T](x: lang.i64) -> lang.i64 where T: Unused {
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
                func calculate(left: lang.i64, right: lang.i64) -> lang.i64
            }
            func doCalc[T](calc: T) -> lang.i64 where T: Calculator {
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
                func calculate(left left: lang.i64, right right: lang.i64) -> lang.i64
            }
            func doCalc[T](calc: T) -> lang.i64 where T: Calculator {
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
                func calculate(left left: lang.i64, right right: lang.i64) -> lang.i64
            }
            func doCalc[T](calc: T) -> lang.i64 where T: Calculator {
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
                let y: lang.i64 = identity[lang.i64, lang.str](42);
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
            func foo(x: lang.i64) -> lang.i64 { return x }
            func main() {
                let y: lang.i64 = foo[lang.i64](42);
            }
        "#,
        )
        .expect(HasError("does not accept type arguments"));
    }

    #[test]
    fn explicit_type_arg_conflicts_with_inferred() {
        // identity[lang.str](42) - 42 is lang.i64 but T=String
        Test::new(
            r#"module Test
            func identity[T](x: T) -> T { return x }
            func main() {
                let y: lang.i64 = identity[lang.str](42);
            }
        "#,
        )
        .expect(HasError("type"));
    }

    #[test]
    fn explicit_type_arg_return_type_mismatch() {
        // identity[lang.str] returns String, but assigned to lang.i64
        Test::new(
            r#"module Test
            func identity[T](x: T) -> T { return x }
            func main() {
                let f = identity[lang.str];
                let x: lang.i64 = f("hello");
            }
        "#,
        )
        .expect(HasError("type"));
    }

    #[test]
    fn generic_struct_init_wrong_field_type() {
        // Box[lang.i64] initialized with String value
        Test::new(
            r#"module Test
            struct Box[T] {
                let value: T
            }
            func main() {
                let b: Box[lang.i64] = Box[lang.i64](value: "hello");
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
                let y: lang.i64 = identity[](42);
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
                let y: lang.i64 = identity[DoesNotExist](42);
            }
        "#,
        )
        .expect(HasError("cannot find type"));
    }

    #[test]
    fn explicit_type_args_on_variable() {
        // x[lang.i64] where x is just an lang.i64 variable
        Test::new(
            r#"module Test
            func main() {
                let x: lang.i64 = 42;
                let y = x[lang.i64];
            }
        "#,
        )
        .expect(HasError("type"));
    }

    #[test]
    fn type_args_on_member_access_of_variable() {
        // self.field[T] should error — brackets on member access are not subscript
        Test::new(
            r#"module Test
            struct Foo {
                var items: Array[lang.i64]
                func bar() -> lang.i64 {
                    return self.items[lang.i64]
                }
            }
        "#,
        )
        .expect(HasError("type"));
    }

    #[test]
    fn explicit_type_arg_not_substituted() {
        // identity[lang.i64](1) + identity[lang.i64](2) should work, but types show as T
        Test::new(
            r#"module Test
            func identity[T](x: T) -> T { return x }
            func main() {
                let x: lang.i64 = lang.i64_add(identity[lang.i64](1), identity[lang.i64](2));
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
                let x: (lang.i64, lang.i64) = identity[(lang.i64, lang.i64)]((1, 2));
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
            func myFunc(x: lang.i64) -> lang.i64 { return x }
            func main() {
                let f: (lang.i64) -> lang.i64 = identity[(lang.i64) -> lang.i64](myFunc);
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn explicit_type_arg_return_type_assigned_wrong() {
        // wrap[lang.i64] returns Box[lang.i64] but assigned to Box[lang.str]
        Test::new(
            r#"module Test
            struct Box[T] {
                let value: T
            }
            func wrap[T](x: T) -> Box[T] { return Box[T](value: x) }
            func main() {
                let b: Box[lang.str] = wrap[lang.i64](42);
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
                let arr: [lang.i64] = identity[[lang.i64]]([1, 2, 3]);
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn explicit_type_args_too_few() {
        // Providing 1 type arg to function that takes 2 should error
        Test::new(
            r#"module Test
            func pair[A, B](a: A, b: B) -> A { return a }
            func main() {
                let x: lang.i64 = pair[lang.i64](1, "hello");
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
                func map[U](x: lang.i64) -> U
            }
            func apply[T](x: T) -> lang.i64 where T: Mapper {
                return x.map[lang.i64](1)
            }
            func main() {}
        "#,
        )
        .expect(Compiles);
    }
}
