//! Multi-file import tests.
//!
//! These tests require multiple source files and can't be expressed
//! as single .ks files. They use the programmatic TestCompiler API.

use kestrel_test_suite::TestCompiler;

// =============================================================================
// BASIC IMPORTS
// =============================================================================

#[test]
fn import_entire_module() {
    let mut tc = TestCompiler::new();
    tc.add_source("library.ks", "module Library\npublic struct PublicClass {}");
    tc.add_source(
        "consumer.ks",
        "module Consumer\nimport Library\nstruct UsesPublic {}",
    );
    tc.expect_no_errors();
}

#[test]
fn import_specific_items() {
    let mut tc = TestCompiler::new();
    tc.add_source(
        "library.ks",
        "module Library\npublic struct PublicClass {}\npublic type PublicAlias = PublicClass;",
    );
    tc.add_source(
        "consumer.ks",
        "module SpecificImport\nimport Library.(PublicClass, PublicAlias)\nstruct MyClass {}",
    );
    tc.expect_no_errors();
}

#[test]
fn import_with_module_alias() {
    let mut tc = TestCompiler::new();
    tc.add_source("library.ks", "module Library\npublic struct PublicClass {}");
    tc.add_source(
        "consumer.ks",
        "module AliasedImport\nimport Library as Lib\nstruct MyClass {}",
    );
    tc.expect_no_errors();
}

#[test]
fn import_with_item_alias() {
    let mut tc = TestCompiler::new();
    tc.add_source("library.ks", "module Library\npublic struct PublicClass {}");
    tc.add_source(
        "consumer.ks",
        "module AliasedImport\nimport Library.(PublicClass as PC)\nstruct MyClass {}",
    );
    tc.expect_no_errors();
}

// =============================================================================
// NESTED MODULES
// =============================================================================

#[test]
fn import_from_nested_module() {
    let mut tc = TestCompiler::new();
    tc.add_source(
        "math_geometry.ks",
        "module Math.Geometry\npublic struct Point {}\npublic struct Circle {}",
    );
    tc.add_source(
        "consumer.ks",
        "module NestedConsumer\nimport Math.Geometry\nstruct MyApp {}",
    );
    tc.expect_no_errors();
}

#[test]
fn import_specific_items_from_nested() {
    let mut tc = TestCompiler::new();
    tc.add_source(
        "math_geometry.ks",
        "module Math.Geometry\npublic struct Point {}\npublic struct Line {}\npublic struct Circle {}",
    );
    tc.add_source(
        "consumer.ks",
        "module NestedConsumer\nimport Math.Geometry.(Point, Circle)\nstruct MyApp {}",
    );
    tc.expect_no_errors();
}

#[test]
fn import_nested_module_with_alias() {
    let mut tc = TestCompiler::new();
    tc.add_source(
        "math_algebra.ks",
        "module Math.Algebra\npublic struct Polynomial {}\npublic struct Equation {}",
    );
    tc.add_source(
        "consumer.ks",
        "module NestedConsumer\nimport Math.Algebra as Alg\nstruct MyApp {}",
    );
    tc.expect_no_errors();
}

// =============================================================================
// VISIBILITY
// =============================================================================

#[test]
fn import_and_verify_public_items_are_accessible() {
    let mut tc = TestCompiler::new();
    tc.add_source(
        "library.ks",
        "module Library\npublic struct PublicClass {}\npublic type PublicAlias = PublicClass;",
    );
    tc.add_source(
        "consumer.ks",
        "module Consumer\nimport Library\nstruct UsesPublic {}",
    );
    tc.expect_no_errors();
}

#[test]
fn internal_symbols_visible_within_same_module() {
    let mut tc = TestCompiler::new();
    tc.add_source(
        "internal_lib.ks",
        "module InternalLib\ninternal struct InternalClass {}\npublic struct PublicClass {}",
    );
    tc.expect_no_errors();
}

// =============================================================================
// CONFLICTS
// =============================================================================

#[test]
fn resolve_naming_conflicts_with_item_aliases() {
    let mut tc = TestCompiler::new();
    tc.add_source(
        "module_a.ks",
        "module ModuleA\npublic struct Widget {}\npublic struct Helper {}",
    );
    tc.add_source(
        "module_b.ks",
        "module ModuleB\npublic struct Widget {}\npublic struct Utility {}",
    );
    tc.add_source(
        "consumer.ks",
        "module AliasedConsumer\nimport ModuleA.(Widget as WidgetA)\nimport ModuleB.(Widget as WidgetB)\nstruct MyClass {}",
    );
    tc.expect_no_errors();
}

// =============================================================================
// ERROR CASES
// =============================================================================

#[test]
fn error_on_importing_nonexistent_module() {
    let mut tc = TestCompiler::new();
    tc.add_source("test.ks", "module Test\nimport NonExistent\nstruct Foo {}");
    tc.expect_error("module 'NonExistent' not found");
}

#[test]
fn error_on_importing_nonexistent_nested_module() {
    let mut tc = TestCompiler::new();
    tc.add_source("library.ks", "module Library\npublic struct Foo {}");
    tc.add_source(
        "consumer.ks",
        "module Consumer\nimport Library.Nonexistent\nstruct Bar {}",
    );
    tc.expect_error("module 'Library.Nonexistent' not found");
}

#[test]
fn error_on_importing_nonexistent_item_from_module() {
    let mut tc = TestCompiler::new();
    tc.add_source("library.ks", "module Library\npublic struct Foo {}");
    tc.add_source(
        "consumer.ks",
        "module Consumer\nimport Library.(Bar)\nstruct Test {}",
    );
    tc.expect_error("symbol 'Bar' not found in module 'Library'");
}

#[test]
fn error_on_importing_nonexistent_item_from_nested_module() {
    let mut tc = TestCompiler::new();
    tc.add_source(
        "math_geometry.ks",
        "module Math.Geometry\npublic struct Point {}",
    );
    tc.add_source(
        "consumer.ks",
        "module Consumer\nimport Math.Geometry.(Circle)\nstruct Test {}",
    );
    tc.expect_error("symbol 'Circle' not found in module 'Math.Geometry'");
}

#[test]
fn error_on_importing_private_item() {
    let mut tc = TestCompiler::new();
    tc.add_source(
        "library.ks",
        "module Library\nprivate struct PrivateClass {}\npublic struct PublicClass {}",
    );
    tc.add_source(
        "consumer.ks",
        "module Consumer\nimport Library.(PrivateClass)\nstruct Test {}",
    );
    tc.expect_error("'PrivateClass' is not accessible");
}

#[test]
fn error_on_duplicate_import_same_item() {
    let mut tc = TestCompiler::new();
    tc.add_source("library.ks", "module Library\npublic struct Foo {}");
    tc.add_source(
        "consumer.ks",
        "module Consumer\nimport Library.(Foo)\nimport Library.(Foo)\nstruct Test {}",
    );
    tc.expect_error("'Foo' is already imported");
}

#[test]
fn error_when_imported_item_conflicts_with_local_declaration() {
    let mut tc = TestCompiler::new();
    tc.add_source("library.ks", "module Library\npublic struct Widget {}");
    tc.add_source(
        "consumer.ks",
        "module Consumer\nimport Library\nstruct Widget {}",
    );
    tc.expect_error("'Widget' is already declared");
}

#[test]
fn error_when_imported_items_conflict_from_different_imports() {
    let mut tc = TestCompiler::new();
    tc.add_source("library_a.ks", "module LibraryA\npublic struct Widget {}");
    tc.add_source("library_b.ks", "module LibraryB\npublic struct Widget {}");
    tc.add_source(
        "consumer.ks",
        "module Consumer\nimport LibraryA.(Widget)\nimport LibraryB\nstruct Test {}",
    );
    tc.expect_error("'Widget' is already imported");
}
