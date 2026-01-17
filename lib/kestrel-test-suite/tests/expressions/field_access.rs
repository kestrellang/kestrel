//! Tests for field access expressions.
//!
//! These tests verify that struct field access is correctly resolved
//! and that appropriate errors are raised for invalid field access.

use kestrel_test_suite::*;

mod field_access {
    use super::*;

    #[test]
    fn simple_field_access_with_struct_verification() {
        Test::new(
            r#"
module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
}

func getX(p: Point) -> lang.i64 {
    p.x
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2)),
        )
        .expect(
            Symbol::new("getX")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn chained_field_access_with_nested_structs() {
        Test::new(
            r#"
module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
}

struct Line {
    let start: Point
    let end: Point
}

func getStartX(line: Line) -> lang.i64 {
    line.start.x
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2)),
        )
        .expect(
            Symbol::new("Line")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2)),
        )
        .expect(
            Symbol::new("getStartX")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn public_field_access_with_multiple_accessors() {
        Test::new(
            r#"
module Main

struct Point {
    public let x: lang.i64
    public let y: lang.i64
}

func getX(p: Point) -> lang.i64 {
    p.x
}

func getY(p: Point) -> lang.i64 {
    p.y
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn field_access_in_variable_declaration() {
        Test::new(
            r#"
module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
}

func example(p: Point) -> lang.i64 {
    let val: lang.i64 = p.x;
    val
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2)),
        )
        .expect(
            Symbol::new("example")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn nonexistent_field_error() {
        Test::new(
            r#"
module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
}

func getZ(p: Point) -> lang.i64 {
    p.z
}
"#,
        )
        .expect(HasError("no member 'z' on type 'Point'"));
    }

    #[test]
    fn member_access_on_primitive_type_error() {
        Test::new(
            r#"
module Main

func test(x: lang.i64) -> lang.i64 {
    x.foo
}
"#,
        )
        .expect(HasError("cannot access member on type"));
    }

    #[test]
    fn private_field_access_error() {
        Test::new(
            r#"
module Main

struct Secret {
    private let hidden: lang.i64
}

func peek(s: Secret) -> lang.i64 {
    s.hidden
}
"#,
        )
        .expect(HasError("is private"));
    }

    #[test]
    fn multiple_field_access_in_single_expression() {
        Test::new(
            r#"
module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
}

func sum(p: Point) -> lang.i64 {
    p.x + p.y
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2)),
        )
        .expect(
            Symbol::new("sum")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }
}
