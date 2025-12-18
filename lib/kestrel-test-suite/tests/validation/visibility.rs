use kestrel_test_suite::*;

#[test]
fn private_method_not_visible_outside_struct() {
    // TODO: Report error: method not visible
    Test::new(
        r#"
        module Main
        struct S {
            private func privateMethod() { }
        }

        func test() {
            let s = S();
            s.privateMethod()
        }
        "#,
    )
    .expect(HasError("is private and not accessible from this scope"));
}

#[test]
fn internal_method_not_visible_outside_module() {
    // TODO: Report error: method not visible
    Test::with_files(&[
        (
            "module_a.ks",
            "module A\npublic struct S {\n    internal func internalMethod() { }\n}",
        ),
        (
            "module_b.ks",
            "module B\nimport A\nfunc test() {\n    let s = S();\n    s.internalMethod()\n}",
        ),
    ])
    .expect(HasError("is internal and not accessible from this scope"));
}

#[test]
fn fileprivate_method_not_visible_outside_file() {
    // Note: Kestrel tests currently use a single module/file per Test instance
    // but the visibility analyzer should still respect Fileprivate.
    // This test might need more sophisticated setup once multi-file tests are supported.
}
