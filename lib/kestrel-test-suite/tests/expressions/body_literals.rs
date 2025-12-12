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

func getInt() -> Int {
    42
}

func getFloat() -> Float {
    3.14
}

func getString() -> String {
    "hello world"
}

func getBool() -> Bool {
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

func getSimpleArray() -> [Int] {
    [1, 2, 3]
}

func getNestedArray() -> [[Int]] {
    [[1, 2], [3, 4]]
}

func getMultiElementArray() -> [Int] {
    [1, 2, 3, 4, 5]
}
"#,
        )
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

func getSimpleTuple() -> (Int, String) {
    (42, "hello")
}

func getNestedTuple() -> ((Int, Int), String) {
    ((1, 2), "point")
}

func getGroupedLiteral() -> Int {
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
