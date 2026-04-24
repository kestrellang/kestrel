//! Multi-file visibility tests.
//!
//! Tests that require multiple modules to verify cross-module visibility rules.

use kestrel_test_suite::TestCompiler;

#[test]
fn internal_method_not_visible_outside_module() {
    let mut tc = TestCompiler::new();
    tc.add_source(
        "module_a.ks",
        "module A\npublic struct S {\n    internal func internalMethod() { }\n}",
    );
    tc.add_source(
        "module_b.ks",
        "module B\nimport A\nfunc test() {\n    let s = S();\n    s.internalMethod()\n}",
    );
    tc.expect_error("is internal and not accessible from this scope");
}

#[test]
fn ambiguous_name_diagnostic() {
    let mut tc = TestCompiler::new();
    tc.add_source("module_a.ks", "module A\npublic struct S { }");
    tc.add_source("module_b.ks", "module B\npublic struct S { }");
    tc.add_source(
        "main.ks",
        "module Main\nimport A\nimport B\nfunc test() {\n    let s = S();\n}",
    );
    tc.expect_error("ambiguous name 'S'");
}
