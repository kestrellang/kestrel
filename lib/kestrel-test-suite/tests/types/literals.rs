use kestrel_test_suite::*;

mod integers {
    use super::*;

    #[test]
    fn integer_decimal() {
        Test::new(
            r#"module Test
            func test() -> Int { 42 }
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
            func decimal() -> Int { 42 }
            func hex_lower() -> Int { 0xff }
            func hex_upper() -> Int { 0XAB }
            func binary() -> Int { 0b1010 }
            func octal() -> Int { 0o755 }
            func zero() -> Int { 0 }
            func large() -> Int { 9223372036854775807 }
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
            func simple() -> Float { 3.14 }
            func zero() -> Float { 0.0 }
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
            func lowercase_positive() -> Float { 1.0e10 }
            func uppercase_negative() -> Float { 2.5E-3 }
            func explicit_positive() -> Float { 1.0e+5 }
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
            func simple() -> String { "hello" }
            func empty() -> String { "" }
            func with_spaces() -> String { "hello world" }
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
            func newline() -> String { "hello\nworld" }
            func tab() -> String { "hello\tworld" }
            func quote() -> String { "say \"hello\"" }
            func backslash() -> String { "path\\to\\file" }
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
            func true_value() -> Bool { true }
            func false_value() -> Bool { false }
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
            func empty() -> [Int] { [] }
            func single() -> [Int] { [1] }
            func multiple() -> [Int] { [1, 2, 3] }
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
            func trailing() -> [Int] { [1, 2, 3,] }
            func nested() -> [[Int]] { [[1, 2], [3, 4]] }
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
            func of_strings() -> [String] { ["hello", "world"] }
            func of_booleans() -> [Bool] { [true, false, true] }
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
            func single() -> (Int,) { (1,) }
            func two_elements() -> (Int, Int) { (1, 2) }
            func multiple() -> (Int, Int, Int) { (1, 2, 3) }
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
            func trailing() -> (Int, Int, Int) { (1, 2, 3,) }
            func nested() -> ((Int, Int), (Int, Int)) { ((1, 2), (3, 4)) }
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
            func mixed_types() -> (Int, String, Bool) { (1, "hello", true) }
            func of_arrays() -> ([Int], [Int]) { ([1, 2], [3, 4]) }
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
            func integer() -> Int { (42) }
            func string() -> String { ("hello") }
            func array() -> [Int] { ([1, 2, 3]) }
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
            func nested() -> Int { ((42)) }
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
            func array_of_tuples() -> [(Int, Int)] { [(1, 2), (3, 4)] }
            func deeply_nested() -> [[(Int,)]] { [[(1,)]] }
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
            func integer() -> Int { 42 }
            func floating() -> Float { 3.14 }
            func text() -> String { "hello" }
            func boolean() -> Bool { true }
            func sequence() -> [Int] { [1, 2, 3] }
            func pair() -> (Int, Int) { (1, 2) }
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
