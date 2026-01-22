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

mod chars {
    use super::*;

    #[test]
    fn char_basic_ascii() {
        Test::new(
            r#"module Test
            func letter() -> lang.i32 { 'a' }
            func uppercase() -> lang.i32 { 'Z' }
            func digit() -> lang.i32 { '0' }
            func space() -> lang.i32 { ' ' }
            func symbol() -> lang.i32 { '!' }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("letter").is(SymbolKind::Function))
        .expect(Symbol::new("uppercase").is(SymbolKind::Function))
        .expect(Symbol::new("digit").is(SymbolKind::Function))
        .expect(Symbol::new("space").is(SymbolKind::Function))
        .expect(Symbol::new("symbol").is(SymbolKind::Function));
    }

    #[test]
    fn char_basic_escapes() {
        Test::new(
            r#"module Test
            func newline() -> lang.i32 { '\n' }
            func tab() -> lang.i32 { '\t' }
            func carriage_return() -> lang.i32 { '\r' }
            func nul_char() -> lang.i32 { '\0' }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("newline").is(SymbolKind::Function))
        .expect(Symbol::new("tab").is(SymbolKind::Function))
        .expect(Symbol::new("carriage_return").is(SymbolKind::Function))
        .expect(Symbol::new("nul_char").is(SymbolKind::Function));
    }

    #[test]
    fn char_quote_escapes() {
        Test::new(
            r#"module Test
            func single_quote() -> lang.i32 { '\'' }
            func double_quote() -> lang.i32 { '\"' }
            func backslash() -> lang.i32 { '\\' }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("single_quote").is(SymbolKind::Function))
        .expect(Symbol::new("double_quote").is(SymbolKind::Function))
        .expect(Symbol::new("backslash").is(SymbolKind::Function));
    }

    #[test]
    fn char_hex_escapes() {
        Test::new(
            r#"module Test
            func null_hex() -> lang.i32 { '\x00' }
            func letter_a() -> lang.i32 { '\x41' }
            func max_ascii() -> lang.i32 { '\x7F' }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("null_hex").is(SymbolKind::Function))
        .expect(Symbol::new("letter_a").is(SymbolKind::Function))
        .expect(Symbol::new("max_ascii").is(SymbolKind::Function));
    }

    #[test]
    fn char_unicode_escapes() {
        Test::new(
            r#"module Test
            func null_unicode() -> lang.i32 { '\u{0}' }
            func letter_a() -> lang.i32 { '\u{41}' }
            func emoji() -> lang.i32 { '\u{1F600}' }
            func max_unicode() -> lang.i32 { '\u{10FFFF}' }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("null_unicode").is(SymbolKind::Function))
        .expect(Symbol::new("letter_a").is(SymbolKind::Function))
        .expect(Symbol::new("emoji").is(SymbolKind::Function))
        .expect(Symbol::new("max_unicode").is(SymbolKind::Function));
    }

    #[test]
    fn char_unicode_without_escapes() {
        // Multi-byte UTF-8 characters that are single code points
        Test::new(
            r#"module Test
            func greek() -> lang.i32 { 'Ω' }
            func cjk() -> lang.i32 { '日' }
            func emoji() -> lang.i32 { '🦅' }
            func precomposed_e() -> lang.i32 { 'é' }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("greek").is(SymbolKind::Function))
        .expect(Symbol::new("cjk").is(SymbolKind::Function))
        .expect(Symbol::new("emoji").is(SymbolKind::Function))
        .expect(Symbol::new("precomposed_e").is(SymbolKind::Function));
    }

    #[test]
    fn char_error_empty() {
        Test::new(
            r#"module Test
            func empty() -> lang.i32 { '' }
        "#,
        )
        .expect(HasError("empty character literal"));
    }

    #[test]
    fn char_error_multiple_ascii() {
        Test::new(
            r#"module Test
            func two_chars() -> lang.i32 { 'ab' }
        "#,
        )
        .expect(HasError("character literal may only contain one codepoint"));
    }

    #[test]
    fn char_error_multiple_chars_three() {
        Test::new(
            r#"module Test
            func three_chars() -> lang.i32 { 'abc' }
        "#,
        )
        .expect(HasError("character literal may only contain one codepoint"));
    }

    #[test]
    fn char_error_multiple_escapes() {
        Test::new(
            r#"module Test
            func two_escapes() -> lang.i32 { '\n\t' }
        "#,
        )
        .expect(HasError("character literal may only contain one codepoint"));
    }

    #[test]
    fn char_error_decomposed_grapheme() {
        // e followed by combining acute accent - two code points that look like one character
        Test::new(
            r#"module Test
            func decomposed_e() -> lang.i32 { 'e\u{0301}' }
        "#,
        )
        .expect(HasError("character literal may only contain one codepoint"));
    }

    #[test]
    fn char_error_family_emoji() {
        // Family emoji is multiple code points joined with ZWJ
        Test::new(
            "module Test\n            func family() -> lang.i32 { '\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}' }\n",
        )
        .expect(HasError("character literal may only contain one codepoint"));
    }

    #[test]
    fn char_error_flag_emoji() {
        // Flag emoji is two regional indicator symbols
        Test::new("module Test\n            func flag() -> lang.i32 { '\u{1F1FA}\u{1F1F8}' }\n")
            .expect(HasError("character literal may only contain one codepoint"));
    }

    #[test]
    fn char_error_invalid_escape() {
        Test::new(
            r#"module Test
            func invalid() -> lang.i32 { '\q' }
        "#,
        )
        .expect(HasError("invalid escape sequence"));
    }

    #[test]
    fn char_error_incomplete_hex() {
        Test::new(
            r#"module Test
            func incomplete() -> lang.i32 { '\x4' }
        "#,
        )
        .expect(HasError("invalid escape sequence"));
    }

    #[test]
    fn char_error_hex_out_of_range() {
        Test::new(
            r#"module Test
            func out_of_range() -> lang.i32 { '\xFF' }
        "#,
        )
        .expect(HasError("ASCII escape"));
    }

    #[test]
    fn char_error_unicode_out_of_range() {
        Test::new(
            r#"module Test
            func out_of_range() -> lang.i32 { '\u{FFFFFF}' }
        "#,
        )
        .expect(HasError("invalid Unicode escape"));
    }

    #[test]
    fn char_error_surrogate_codepoint() {
        // Surrogates (0xD800-0xDFFF) are invalid Unicode scalars
        Test::new(
            r#"module Test
            func surrogate() -> lang.i32 { '\u{D800}' }
        "#,
        )
        .expect(HasError("invalid Unicode escape"));
    }

    // Char integration tests
    #[test]
    fn char_explicit_type() {
        // Char literal with explicit Char type
        Test::new(
            r#"module Test
            func get_a() -> std.text.Char { 'a' }
        "#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn char_default_type() {
        // Char literals default to Char when stdlib is available
        Test::new(
            r#"module Test
            func test() {
                let c = 'x';
                let _: std.text.Char = c;
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn char_comparison() {
        // Char supports comparison since it's Equatable
        Test::new(
            r#"module Test
            func is_space(c: std.text.Char) -> std.core.Bool {
                c == ' '
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn char_escapes() {
        // Various escape sequences work with Char
        Test::new(
            r#"module Test
            func newline() -> std.text.Char { '\n' }
            func tab() -> std.text.Char { '\t' }
            func nul() -> std.text.Char { '\0' }
        "#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}
