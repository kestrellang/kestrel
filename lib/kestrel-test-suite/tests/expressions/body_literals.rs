//! Tests for literal expressions in function bodies.
//!
//! These tests verify that various literal values (integers, floats, strings, booleans, unit,
//! arrays, tuples) are correctly resolved when used in function bodies.

use kestrel_test_suite::*;

mod literal_expressions {
    use super::*;

    #[test]
    fn primitive_literals_in_bodies() {
        Test::new(
            r#"
module Main

func getInt() -> lang.i64 {
    42
}

func getFloat() -> lang.f64 {
    3.14
}

func getString() -> lang.str {
    "hello world"
}

func getBool() -> lang.i1 {
    true
}

func getUnit() -> () {
    ()
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("getInt")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        )
        .expect(
            Symbol::new("getFloat")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        )
        .expect(
            Symbol::new("getString")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        )
        .expect(
            Symbol::new("getBool")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        )
        .expect(
            Symbol::new("getUnit")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        );
    }
}

mod composite_expressions {
    use super::*;

    #[test]
    fn array_literals_in_bodies() {
        Test::new(
            r#"
module Main

func getSimpleArray() -> [lang.i64] {
    [1, 2, 3]
}

func getNestedArray() -> [[lang.i64]] {
    [[1, 2], [3, 4]]
}

func getMultiElementArray() -> [lang.i64] {
    [1, 2, 3, 4, 5]
}
"#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(
            Symbol::new("getSimpleArray")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        )
        .expect(
            Symbol::new("getNestedArray")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        )
        .expect(
            Symbol::new("getMultiElementArray")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        );
    }

    #[test]
    fn tuple_literals_in_bodies() {
        Test::new(
            r#"
module Main

func getSimpleTuple() -> (lang.i64, lang.str) {
    (42, "hello")
}

func getNestedTuple() -> ((lang.i64, lang.i64), lang.str) {
    ((1, 2), "point")
}

func getGroupedLiteral() -> lang.i64 {
    (42)
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("getSimpleTuple")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        )
        .expect(
            Symbol::new("getNestedTuple")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        )
        .expect(
            Symbol::new("getGroupedLiteral")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        );
    }
}
