use kestrel_test_suite::*;

#[test]
fn null_assignable_to_optional_type() {
    // null uses ExpressibleByNullLiteral protocol, stdlib provides Optional implementation
    Test::new(
        r#"
        module Main
        import std.num.Int64
        func test() {
            let x: Int64? = null;
        }
        "#,
    )
    .with_stdlib()
    .expect(Compiles);
}

#[test]
fn non_optional_type_cannot_be_null() {
    // lang.i64 does not conform to ExpressibleByNullLiteral
    Test::new(
        r#"
        module Main
        func test() {
            let x: lang.i64 = null;
        }
        "#,
    )
    .with_stdlib()
    .expect(HasError("does not conform"));
}
