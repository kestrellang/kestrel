use kestrel_test_suite::*;

#[test]
fn null_assignable_to_optional_type() {
    // TODO: Handle null properly with optional types
    Test::new(
        r#"
        module Main
        func test() {
            let x: Int? = null;
        }
        "#,
    )
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
            let x: Int = null;
        }
        "#,
    )
    .expect(HasError("cannot assign null to non-optional type"));
}

