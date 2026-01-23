use kestrel_test_suite::*;

#[test]
fn null_assignable_to_optional_type() {
    // null desugars to .None, so stdlib is required for Optional enum
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
#[ignore]
fn non_optional_type_cannot_be_null() {
    // TODO: Handle null properly with optional types
    Test::new(
        r#"
        module Main
        func test() {
            let x: lang.i64 = null;
        }
        "#,
    )
    .expect(HasError("cannot assign null to non-optional type"));
}
