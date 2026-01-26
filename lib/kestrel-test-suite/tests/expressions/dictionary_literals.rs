//! Tests for dictionary literal expressions.
//!
//! Dictionary literals use the syntax `[key: value, ...]` for non-empty dictionaries
//! and `[:]` for empty dictionaries. They work through the `ExpressibleByDictionaryLiteral`
//! protocol system.

use kestrel_test_suite::*;

mod basic {
    use super::*;

    #[test]
    fn empty_dictionary_with_type_annotation() {
        Test::new(
            r#"
module Main

func getEmpty() -> [String: Int] {
    [:]
}
"#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(
            Symbol::new("getEmpty")
                .is(SymbolKind::Function)
                .has(Behavior::HasBody(true)),
        );
    }

    #[test]
    fn single_pair_dictionary() {
        Test::new(
            r#"
module Main

func getSingle() -> [String: Int] {
    ["key": 42]
}
"#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(
            Symbol::new("getSingle")
                .is(SymbolKind::Function)
                .has(Behavior::HasBody(true)),
        );
    }

    #[test]
    fn multi_pair_dictionary() {
        Test::new(
            r#"
module Main

func getMulti() -> [String: Int] {
    ["alice": 30, "bob": 25, "charlie": 35]
}
"#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(
            Symbol::new("getMulti")
                .is(SymbolKind::Function)
                .has(Behavior::HasBody(true)),
        );
    }

    #[test]
    fn trailing_comma_allowed() {
        Test::new(
            r#"
module Main

func getTrailing() -> [String: Int] {
    ["a": 1, "b": 2,]
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

mod type_inference {
    use super::*;

    #[test]
    fn infer_from_return_type() {
        Test::new(
            r#"
module Main

func getData() -> [String: Int] {
    ["x": 1, "y": 2]
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn infer_from_variable_annotation() {
        Test::new(
            r#"
module Main

func test() {
    let d: [String: Int] = ["a": 1, "b": 2];
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn infer_from_parameter_type() {
        Test::new(
            r#"
module Main

func process(data: [String: Int]) { }

func test() {
    process(["key": 42])
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn infer_empty_from_context() {
        Test::new(
            r#"
module Main

func process(data: [String: Int]) { }

func test() {
    process([:])
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

mod nested {
    use super::*;

    #[test]
    fn nested_dictionary_literals() {
        Test::new(
            r#"
module Main

func getNested() -> [String: [String: Int]] {
    ["outer": ["inner": 42]]
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn dictionary_with_array_values() {
        Test::new(
            r#"
module Main

func getArrayValues() -> [String: [Int]] {
    ["numbers": [1, 2, 3], "more": [4, 5, 6]]
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

mod computed_keys {
    use super::*;

    #[test]
    fn variable_as_key() {
        Test::new(
            r#"
module Main

func test() {
    let key = "mykey";
    let d: [String: Int] = [key: 42];
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn function_call_as_key() {
        Test::new(
            r#"
module Main

func getKey() -> String { "computed" }

func test() {
    let d: [String: Int] = [getKey(): 42];
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

mod disambiguation {
    use super::*;

    #[test]
    fn empty_array_vs_empty_dict() {
        // [] is empty array, [:] is empty dictionary
        Test::new(
            r#"
module Main

func getArray() -> [Int] {
    []
}

func getDict() -> [String: Int] {
    [:]
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn single_element_array_vs_single_pair_dict() {
        Test::new(
            r#"
module Main

func getArray() -> [Int] {
    [42]
}

func getDict() -> [String: Int] {
    ["key": 42]
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

mod errors {
    use super::*;

    #[test]
    fn empty_dict_without_context() {
        Test::new(
            r#"
module Main

func test() {
    let d = [:];
}
"#,
        )
        .with_stdlib()
        .expect(HasError("cannot infer"));
    }

    #[test]
    fn key_type_mismatch() {
        Test::new(
            r#"
module Main

func test() {
    let d: [String: Int] = [42: 1];
}
"#,
        )
        .with_stdlib()
        .expect(HasError("cannot convert"));
    }

    #[test]
    fn value_type_mismatch() {
        Test::new(
            r#"
module Main

func test() {
    let d: [String: Int] = ["key": "value"];
}
"#,
        )
        .with_stdlib()
        .expect(HasError("cannot convert"));
    }

    #[test]
    fn inconsistent_key_types() {
        Test::new(
            r#"
module Main

func test() {
    let d: [String: Int] = ["key": 1, 42: 2];
}
"#,
        )
        .with_stdlib()
        .expect(HasError("cannot convert"));
    }

    #[test]
    fn inconsistent_value_types() {
        Test::new(
            r#"
module Main

func test() {
    let d: [String: Int] = ["a": 1, "b": "two"];
}
"#,
        )
        .with_stdlib()
        .expect(HasError("cannot convert"));
    }
}
