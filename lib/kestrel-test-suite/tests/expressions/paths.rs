use kestrel_test_suite::*;

/// Macro for simple compilation tests that wrap code in a function body
macro_rules! compiles {
    ($name:ident, $code:expr) => {
        #[test]
        fn $name() {
            Test::new(concat!("module Test\nfunc test() {\n", $code, "\n}"))
                .expect(Compiles)
                .expect(Symbol::new("Test.test").is(SymbolKind::Function));
        }
    };
}

/// Macro for tests with custom function signatures
macro_rules! compiles_fn {
    ($name:ident, $sig:expr, $body:expr) => {
        #[test]
        fn $name() {
            Test::new(concat!(
                "module Test\nfunc test",
                $sig,
                " {\n",
                $body,
                "\n}"
            ))
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function));
        }
    };
}

mod path_expressions {
    use super::*;

    // Note: Path expressions require defined names, so we use parameters
    compiles_fn!(path_single_segment, "(foo: Int) -> Int", "foo");

    #[test]
    fn paths_in_containers() {
        Test::new(
            "module Test\nfunc test(foo: Int, bar: Int) -> Int {\n[foo, bar];\n(foo, bar);\n(foo)\n}",
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Test.test")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(2)),
        );
    }
}

mod variable_declarations {
    use super::*;

    compiles!(let_with_type, "let x: Int = 42;");
    compiles!(var_with_type, "var x: Int = 42;");
    compiles!(let_without_initializer, "let x: Int;");

    #[test]
    fn multiple_declarations() {
        Test::new(
            "module Test\nfunc test() {\nlet x: Int = 1;\nlet y: Int = 2;\nlet z: Int = 3;\n}",
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Test.test")
                .is(SymbolKind::Function)
                .has(Behavior::HasBody(true)),
        );
    }

    compiles_fn!(let_with_path_initializer, "(foo: Int)", "let x: Int = foo;");
    compiles!(
        let_with_complex_type,
        r#"let x: (Int, String) = (1, "hello");"#
    );
    compiles!(let_with_array_type, "let x: [Int] = [1, 2, 3];");
}

mod variable_shadowing {
    use super::*;

    compiles_fn!(let_shadows_parameter, "(x: Int)", "let x: Int = 42;");

    #[test]
    fn sequential_shadowing() {
        Test::new(
            "module Test\nfunc test() {\nlet x: Int = 1;\nlet x: Int = 2;\nlet x: Int = 3;\n}",
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Test.test")
                .is(SymbolKind::Function)
                .has(Behavior::HasBody(true)),
        );
    }
}

mod parameter_usage {
    use super::*;

    compiles_fn!(use_single_parameter, "(x: Int) -> Int", "x");

    #[test]
    fn multiple_parameters_usage() {
        Test::new("module Test\nfunc test(x: Int, label y: Int) {\nx;\ny;\n}")
            .expect(Compiles)
            .expect(
                Symbol::new("Test.test")
                    .is(SymbolKind::Function)
                    .has(Behavior::ParameterCount(2)),
            );
    }
}

mod expression_statements {
    use super::*;

    compiles!(expression_statement_literal, "42;");
    compiles_fn!(expression_statement_path, "(foo: Int)", "foo;");

    #[test]
    fn multiple_expression_statements() {
        Test::new("module Test\nfunc test() {\n42;\n\"hello\";\ntrue;\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function));
    }
}

mod complex_expressions {
    use super::*;

    #[test]
    fn nested_containers() {
        Test::new("module Test\nfunc test() {\n[[1, 2], [3, 4]];\n((1, 2), (3, 4));\n[(1, 2), (3, 4)];\n([1, 2], [3, 4]);\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function));
    }

    compiles!(deeply_nested_grouping, "((((42))));");
    compiles!(mixed_literals, r#"(42, 3.14, "hello", true, false);"#);
    #[test]
    fn empty_array_requires_type_annotation() {
        // Empty array without context cannot infer element type
        Test::new("module Test\nfunc test() {\n[];\n}")
            .expect(HasError("could not infer type"));
    }
    compiles!(empty_tuple_is_unit, "();");
    compiles!(single_element_tuple, "(42,);");

    #[test]
    fn complex_type_declarations() {
        Test::new("module Test\nfunc test() {\nlet x: [[Int]] = [[1, 2], [3, 4]];\nlet pair: (Int, Int) = (1, 2);\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function));
    }

    compiles_fn!(
        let_with_function_type,
        "(foo: (Int) -> Int)",
        "let f: (Int) -> Int = foo;"
    );

    #[test]
    fn numeric_literal_variants() {
        Test::new(
            "module Test\nfunc test() {\n0xFF;\n0b1010;\n0o777;\n1.5e10;\n1_000_000;\n0xFF_FF;\n}",
        )
        .expect(Compiles)
        .expect(Symbol::new("Test.test").is(SymbolKind::Function));
    }

    compiles!(string_with_escapes, r#""hello\nworld";"#);
}

mod edge_cases {
    use super::*;

    #[test]
    fn trailing_commas() {
        Test::new("module Test\nfunc test() {\n[1, 2, 3,];\n(1, 2, 3,);\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function));
    }

    #[test]
    fn unary_operators_on_literals() {
        Test::new("module Test\nfunc test() {\n-42;\n-3.14;\n!true;\n!!false;\n--42;\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function));
    }

    #[test]
    fn unary_operators_on_paths() {
        Test::new("module Test\nfunc test(x: Int, b: Bool) {\n-x;\n!b;\n-!-!x;\n}")
            .expect(Compiles)
            .expect(
                Symbol::new("Test.test")
                    .is(SymbolKind::Function)
                    .has(Behavior::ParameterCount(2)),
            );
    }

    #[test]
    fn unary_in_containers() {
        Test::new("module Test\nfunc test() {\n[-1, -2, -3];\n(-1, -2);\n[([(-1,)],)];\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function));
    }

    #[test]
    fn numeric_zero_variants() {
        Test::new("module Test\nfunc test() {\n0;\n0x0;\n0b0;\n0o0;\n0.0;\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function));
    }

    #[test]
    fn scientific_notation_variants() {
        Test::new("module Test\nfunc test() {\n1.5e10;\n1.0e-10;\n1.0e+10;\n1.0E10;\n-1.0e10;\n-1.0e-10;\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function));
    }

    compiles!(very_large_integer, "999_999_999_999_999_999;");

    #[test]
    fn special_string_content() {
        Test::new("module Test\nfunc test() {\n\"Hello 世界 🌍\";\n\"\";\n\"hello\\nworld\";\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function));
    }

    #[test]
    fn whitespace_handling() {
        Test::new("module Test\nfunc test() {\n[   1   ,   2   ,   3   ];\n[\n    1,\n    2,\n    3\n];\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function));
    }

    #[test]
    fn special_types() {
        Test::new("module Test\nfunc test(foo: !) {\nlet x: ! = foo;\nlet y: () = ();\n}")
            .expect(Compiles)
            .expect(
                Symbol::new("Test.test")
                    .is(SymbolKind::Function)
                    .has(Behavior::ParameterCount(1)),
            );
    }

    #[test]
    fn underscore_and_unicode_identifiers() {
        Test::new("module Test\nfunc test(_private: Int, café: Int) {\n_private;\ncafé;\n}")
            .expect(Compiles)
            .expect(
                Symbol::new("Test.test")
                    .is(SymbolKind::Function)
                    .has(Behavior::ParameterCount(2)),
            );
    }

    #[test]
    fn deeply_nested_structures() {
        Test::new("module Test\nfunc test() {\n[[[1]]];\n((((((1,),),),),),);\nlet x: [[[Int]]] = [[[1]]];\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function));
    }

    #[test]
    fn complex_function_types() {
        Test::new("module Test\nfunc test(foo: (Int) -> (Int) -> Int) {\nlet f: (Int) -> (Int) -> Int = foo;\nlet fs: [(Int) -> Int] = [];\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function).has(Behavior::ParameterCount(1)));
    }

    #[test]
    fn variable_shadowing_edge_case() {
        Test::new("module Test\nfunc test() {\nlet x: Int = 1;\nlet x: Int = 2;\nlet x: Int = 3;\nlet x: Int = 4;\nlet x: Int = 5;\nlet x: Int = 6;\nlet x: Int = 7;\nlet x: Int = 8;\nlet x: Int = 9;\nlet x: Int = 10;\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function));
    }

    compiles_fn!(parameter_with_same_name_as_type, "(Int: Int) -> Int", "Int");

    #[test]
    fn null_literals() {
        Test::new("module Test\nfunc test() {\nnull;\n[null, null, null];\n(null, 42, null);\n[null, -1, null, -2, null, -3];\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function));
    }

    #[test]
    fn declarations_with_unary_initializers() {
        Test::new("module Test\nfunc test(foo: Int) {\nlet x: Int = -foo;\nlet y: (Int, Int) = (-1, -2);\nlet z: [Int] = [-1, -2, -3];\nlet w: (Int, Bool, Int) = (-1, !true, --2);\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function).has(Behavior::ParameterCount(1)));
    }

    #[test]
    fn many_variable_declarations() {
        Test::new("module Test\nfunc test() {\nlet a: Int = 1;\nlet b: Float = 2.0;\nlet c: String = \"hello\";\nlet d: Bool = true;\nlet e: [Int] = [1, 2, 3];\nlet f: (Int, Int) = (1, 2);\nlet g: () = ();\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function));
    }

    #[test]
    fn function_with_many_parameters() {
        Test::new("module Test\nfunc test(a: Int, b: Int, c: Int, d: Int, e: Int, f: Int, g: Int, h: Int) -> Int {\na;\nb;\nc;\nd;\ne;\nf;\ng;\nh\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function).has(Behavior::ParameterCount(8)));
    }

    #[test]
    fn labeled_parameters() {
        Test::new("module Test\nfunc test(label1 a: Int, b: Int, label2 c: Int, d: Int) -> (Int, Int, Int, Int) {\n(a, b, c, d)\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function).has(Behavior::ParameterCount(4)));
    }

    compiles_fn!(
        function_type_with_many_params,
        "(foo: (Int, Int, Int, Int) -> (Int, Int))",
        "let f: (Int, Int, Int, Int) -> (Int, Int) = foo;"
    );

    #[test]
    fn mixed_statements_and_expressions() {
        Test::new("module Test\nfunc test() -> Int {\nlet x: Int = 1;\n42;\nlet a: Int = 1;\n42;\nlet b: Int = 2;\n\"hello\";\nlet c: Int = 3;\nc\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function));
    }

    #[test]
    fn comments_in_code() {
        Test::new("module Test\nfunc test() -> Int {\n// This is a comment\n42;\n[1, /* comment */ 2, 3];\n/* outer /* inner */ still outer */\n42\n}")
            .expect(Compiles)
            .expect(Symbol::new("Test.test").is(SymbolKind::Function));
    }
}
