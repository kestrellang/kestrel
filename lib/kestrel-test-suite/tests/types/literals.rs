use kestrel_test_suite::*;

mod integers {
    use super::*;

    #[test]
    fn integer_decimal() {
        Test::new(
            r#"module Test
            func test() -> lang.i64 { 42 }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("test")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        );
    }

    #[test]
    fn integer_formats() {
        // Test that various integer literal formats all work
        Test::new(
            r#"module Test
            func decimal() -> lang.i64 { 42 }
            func hex_lower() -> lang.i64 { 0xff }
            func hex_upper() -> lang.i64 { 0XAB }
            func binary() -> lang.i64 { 0b1010 }
            func octal() -> lang.i64 { 0o755 }
            func zero() -> lang.i64 { 0 }
            func large() -> lang.i64 { 9223372036854775807 }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("decimal").is(SymbolKind::Function))
        .expect(Symbol::new("hex_lower").is(SymbolKind::Function))
        .expect(Symbol::new("hex_upper").is(SymbolKind::Function))
        .expect(Symbol::new("binary").is(SymbolKind::Function))
        .expect(Symbol::new("octal").is(SymbolKind::Function))
        .expect(Symbol::new("zero").is(SymbolKind::Function))
        .expect(Symbol::new("large").is(SymbolKind::Function));
    }
}

mod floats {
    use super::*;

    #[test]
    fn float_basic_forms() {
        Test::new(
            r#"module Test
            func simple() -> lang.f64 { 3.14 }
            func zero() -> lang.f64 { 0.0 }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("simple").is(SymbolKind::Function))
        .expect(Symbol::new("zero").is(SymbolKind::Function));
    }

    #[test]
    fn float_scientific_notation() {
        // Test scientific notation with various exponent formats
        Test::new(
            r#"module Test
            func lowercase_positive() -> lang.f64 { 1.0e10 }
            func uppercase_negative() -> lang.f64 { 2.5E-3 }
            func explicit_positive() -> lang.f64 { 1.0e+5 }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("lowercase_positive").is(SymbolKind::Function))
        .expect(Symbol::new("uppercase_negative").is(SymbolKind::Function))
        .expect(Symbol::new("explicit_positive").is(SymbolKind::Function));
    }
}

mod strings {
    use super::*;

    #[test]
    fn string_basic_forms() {
        Test::new(
            r#"module Test
            func simple() -> lang.str { "hello" }
            func empty() -> lang.str { "" }
            func with_spaces() -> lang.str { "hello world" }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("simple").is(SymbolKind::Function))
        .expect(Symbol::new("empty").is(SymbolKind::Function))
        .expect(Symbol::new("with_spaces").is(SymbolKind::Function));
    }

    #[test]
    fn string_escape_sequences() {
        // Test common escape sequences
        Test::new(
            r#"module Test
            func newline() -> lang.str { "hello\nworld" }
            func tab() -> lang.str { "hello\tworld" }
            func quote() -> lang.str { "say \"hello\"" }
            func backslash() -> lang.str { "path\\to\\file" }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("newline").is(SymbolKind::Function))
        .expect(Symbol::new("tab").is(SymbolKind::Function))
        .expect(Symbol::new("quote").is(SymbolKind::Function))
        .expect(Symbol::new("backslash").is(SymbolKind::Function));
    }
}

mod booleans {
    use super::*;

    #[test]
    fn boolean_literals() {
        Test::new(
            r#"module Test
            func true_value() -> lang.i1 { true }
            func false_value() -> lang.i1 { false }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("true_value").is(SymbolKind::Function))
        .expect(Symbol::new("false_value").is(SymbolKind::Function));
    }
}

mod arrays {
    use super::*;

    #[test]
    fn array_basic_forms() {
        Test::new(
            r#"module Test
            func empty() -> [lang.i64] { [] }
            func single() -> [lang.i64] { [1] }
            func multiple() -> [lang.i64] { [1, 2, 3] }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("empty").is(SymbolKind::Function))
        .expect(Symbol::new("single").is(SymbolKind::Function))
        .expect(Symbol::new("multiple").is(SymbolKind::Function));
    }

    #[test]
    fn array_trailing_comma_and_nesting() {
        Test::new(
            r#"module Test
            func trailing() -> [lang.i64] { [1, 2, 3,] }
            func nested() -> [[lang.i64]] { [[1, 2], [3, 4]] }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("trailing").is(SymbolKind::Function))
        .expect(Symbol::new("nested").is(SymbolKind::Function));
    }

    #[test]
    fn array_of_various_types() {
        // Test arrays containing strings and booleans
        Test::new(
            r#"module Test
            func of_strings() -> [lang.str] { ["hello", "world"] }
            func of_booleans() -> [lang.i1] { [true, false, true] }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("of_strings").is(SymbolKind::Function))
        .expect(Symbol::new("of_booleans").is(SymbolKind::Function));
    }

    #[test]
    fn array_mixed_types_error() {
        // Arrays with mixed element types should produce an error
        Test::new(
            r#"module Test
            func mixed_types() { [1, "hello", true] }
        "#,
        )
        .expect(HasError("array element type mismatch"));
    }
}

mod tuples {
    use super::*;

    #[test]
    fn tuple_basic_forms() {
        // Test single element (with trailing comma) and multi-element tuples
        Test::new(
            r#"module Test
            func single() -> (lang.i64,) { (1,) }
            func two_elements() -> (lang.i64, lang.i64) { (1, 2) }
            func multiple() -> (lang.i64, lang.i64, lang.i64) { (1, 2, 3) }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("single").is(SymbolKind::Function))
        .expect(Symbol::new("two_elements").is(SymbolKind::Function))
        .expect(Symbol::new("multiple").is(SymbolKind::Function));
    }

    #[test]
    fn tuple_trailing_comma_and_nesting() {
        Test::new(
            r#"module Test
            func trailing() -> (lang.i64, lang.i64, lang.i64) { (1, 2, 3,) }
            func nested() -> ((lang.i64, lang.i64), (lang.i64, lang.i64)) { ((1, 2), (3, 4)) }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("trailing").is(SymbolKind::Function))
        .expect(Symbol::new("nested").is(SymbolKind::Function));
    }

    #[test]
    fn tuple_complex_content() {
        Test::new(
            r#"module Test
            func mixed_types() -> (lang.i64, lang.str, lang.i1) { (1, "hello", true) }
            func of_arrays() -> ([lang.i64], [lang.i64]) { ([1, 2], [3, 4]) }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("mixed_types").is(SymbolKind::Function))
        .expect(Symbol::new("of_arrays").is(SymbolKind::Function));
    }
}

mod grouping {
    use super::*;

    #[test]
    fn grouping_parentheses() {
        // Single element without trailing comma is grouping (not a tuple)
        Test::new(
            r#"module Test
            func integer() -> lang.i64 { (42) }
            func string() -> lang.str { ("hello") }
            func array() -> [lang.i64] { ([1, 2, 3]) }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("integer").is(SymbolKind::Function))
        .expect(Symbol::new("string").is(SymbolKind::Function))
        .expect(Symbol::new("array").is(SymbolKind::Function));
    }

    #[test]
    fn grouping_nested() {
        Test::new(
            r#"module Test
            func nested() -> lang.i64 { ((42)) }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("nested").is(SymbolKind::Function));
    }
}

mod unit {
    use super::*;

    #[test]
    fn unit_type() {
        Test::new(
            r#"module Test
            func unit_value() { () }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("unit_value").is(SymbolKind::Function));
    }
}

mod complex {
    use super::*;

    #[test]
    fn complex_nested_structures() {
        // Test deeply nested and complex combinations of containers
        Test::new(
            r#"module Test
            func array_of_tuples() -> [(lang.i64, lang.i64)] { [(1, 2), (3, 4)] }
            func deeply_nested() -> [[(lang.i64,)]] { [[(1,)]] }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("array_of_tuples").is(SymbolKind::Function))
        .expect(Symbol::new("deeply_nested").is(SymbolKind::Function));
    }

    #[test]
    fn all_literal_types_in_module() {
        // Comprehensive test that functions can contain all literal types
        Test::new(
            r#"module Test
            func integer() -> lang.i64 { 42 }
            func floating() -> lang.f64 { 3.14 }
            func text() -> lang.str { "hello" }
            func boolean() -> lang.i1 { true }
            func sequence() -> [lang.i64] { [1, 2, 3] }
            func pair() -> (lang.i64, lang.i64) { (1, 2) }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("integer")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        )
        .expect(
            Symbol::new("floating")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        )
        .expect(
            Symbol::new("text")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        )
        .expect(
            Symbol::new("boolean")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        )
        .expect(
            Symbol::new("sequence")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        )
        .expect(
            Symbol::new("pair")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        );
    }
}
