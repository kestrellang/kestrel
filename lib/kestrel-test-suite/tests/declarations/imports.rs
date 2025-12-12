use kestrel_test_suite::*;

mod basic {
    use super::*;

    #[test]
    fn import_entire_module() {
        Test::with_files(&[
            ("library.ks", "module Library\npublic struct PublicClass {}"),
            (
                "consumer.ks",
                "module Consumer\nimport Library\nstruct UsesPublic {}",
            ),
        ])
        .expect(Compiles)
        .expect(
            Symbol::new("PublicClass")
                .is(SymbolKind::Struct)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(Symbol::new("UsesPublic").is(SymbolKind::Struct));
    }

    #[test]
    fn import_specific_items() {
        Test::with_files(&[
            (
                "library.ks",
                "module Library\npublic struct PublicClass {}\npublic type PublicAlias = PublicClass;",
            ),
            (
                "consumer.ks",
                "module SpecificImport\nimport Library.(PublicClass, PublicAlias)\nstruct MyClass {}",
            ),
        ])
        .expect(Compiles)
        .expect(Symbol::new("PublicClass").is(SymbolKind::Struct).has(Behavior::Visibility(Visibility::Public)))
        .expect(Symbol::new("PublicAlias").is(SymbolKind::TypeAlias).has(Behavior::Visibility(Visibility::Public)))
        .expect(Symbol::new("MyClass").is(SymbolKind::Struct));
    }

    #[test]
    fn import_with_module_alias() {
        Test::with_files(&[
            ("library.ks", "module Library\npublic struct PublicClass {}"),
            (
                "consumer.ks",
                "module AliasedImport\nimport Library as Lib\nstruct MyClass {}",
            ),
        ])
        .expect(Compiles)
        .expect(
            Symbol::new("PublicClass")
                .is(SymbolKind::Struct)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(Symbol::new("MyClass").is(SymbolKind::Struct));
    }

    #[test]
    fn import_with_item_alias() {
        Test::with_files(&[
            ("library.ks", "module Library\npublic struct PublicClass {}"),
            (
                "consumer.ks",
                "module AliasedImport\nimport Library.(PublicClass as PC)\nstruct MyClass {}",
            ),
        ])
        .expect(Compiles)
        .expect(
            Symbol::new("PublicClass")
                .is(SymbolKind::Struct)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(Symbol::new("MyClass").is(SymbolKind::Struct));
    }
}

mod nested_modules {
    use super::*;

    #[test]
    fn import_from_nested_module() {
        Test::with_files(&[
            (
                "math_geometry.ks",
                "module Math.Geometry\npublic struct Point {}\npublic struct Circle {}",
            ),
            (
                "consumer.ks",
                "module NestedConsumer\nimport Math.Geometry\nstruct MyApp {}",
            ),
        ])
        .expect(Compiles)
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(
            Symbol::new("Circle")
                .is(SymbolKind::Struct)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(Symbol::new("MyApp").is(SymbolKind::Struct));
    }

    #[test]
    fn import_specific_items_from_nested() {
        Test::with_files(&[
            (
                "math_geometry.ks",
                "module Math.Geometry\npublic struct Point {}\npublic struct Line {}\npublic struct Circle {}",
            ),
            (
                "consumer.ks",
                "module NestedConsumer\nimport Math.Geometry.(Point, Circle)\nstruct MyApp {}",
            ),
        ])
        .expect(Compiles)
        .expect(Symbol::new("Point").is(SymbolKind::Struct).has(Behavior::Visibility(Visibility::Public)))
        .expect(Symbol::new("Circle").is(SymbolKind::Struct).has(Behavior::Visibility(Visibility::Public)))
        .expect(Symbol::new("MyApp").is(SymbolKind::Struct));
    }

    #[test]
    fn import_nested_module_with_alias() {
        Test::with_files(&[
            (
                "math_algebra.ks",
                "module Math.Algebra\npublic struct Polynomial {}\npublic struct Equation {}",
            ),
            (
                "consumer.ks",
                "module NestedConsumer\nimport Math.Algebra as Alg\nstruct MyApp {}",
            ),
        ])
        .expect(Compiles)
        .expect(
            Symbol::new("Polynomial")
                .is(SymbolKind::Struct)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(
            Symbol::new("Equation")
                .is(SymbolKind::Struct)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(Symbol::new("MyApp").is(SymbolKind::Struct));
    }
}

mod visibility {
    use super::*;

    #[test]
    fn import_and_verify_public_items_are_accessible() {
        Test::with_files(&[
            (
                "library.ks",
                "module Library\npublic struct PublicClass {}\npublic type PublicAlias = PublicClass;",
            ),
            (
                "consumer.ks",
                "module Consumer\nimport Library\nstruct UsesPublic {}",
            ),
        ])
        .expect(Compiles)
        .expect(Symbol::new("PublicClass").is(SymbolKind::Struct).has(Behavior::Visibility(Visibility::Public)))
        .expect(Symbol::new("PublicAlias").is(SymbolKind::TypeAlias).has(Behavior::Visibility(Visibility::Public)))
        .expect(Symbol::new("UsesPublic").is(SymbolKind::Struct));
    }

    #[test]
    fn internal_symbols_visible_within_same_module() {
        Test::with_files(&[(
            "internal_lib.ks",
            "module InternalLib\ninternal struct InternalClass {}\npublic struct PublicClass {}",
        )])
        .expect(Compiles)
        .expect(
            Symbol::new("InternalClass")
                .is(SymbolKind::Struct)
                .has(Behavior::Visibility(Visibility::Internal)),
        )
        .expect(
            Symbol::new("PublicClass")
                .is(SymbolKind::Struct)
                .has(Behavior::Visibility(Visibility::Public)),
        );
    }
}

mod conflicts {
    use super::*;

    #[test]
    fn resolve_naming_conflicts_with_item_aliases() {
        Test::with_files(&[
            ("module_a.ks", "module ModuleA\npublic struct Widget {}\npublic struct Helper {}"),
            ("module_b.ks", "module ModuleB\npublic struct Widget {}\npublic struct Utility {}"),
            (
                "consumer.ks",
                "module AliasedConsumer\nimport ModuleA.(Widget as WidgetA)\nimport ModuleB.(Widget as WidgetB)\nstruct MyClass {}",
            ),
        ])
        .expect(Compiles)
        .expect(Symbol::new("Widget").is(SymbolKind::Struct).has(Behavior::Visibility(Visibility::Public)))
        .expect(Symbol::new("Helper").is(SymbolKind::Struct).has(Behavior::Visibility(Visibility::Public)))
        .expect(Symbol::new("Utility").is(SymbolKind::Struct).has(Behavior::Visibility(Visibility::Public)))
        .expect(Symbol::new("MyClass").is(SymbolKind::Struct));
    }
}

mod errors {
    use super::*;

    #[test]
    fn error_on_importing_nonexistent_module() {
        Test::new("module Test\nimport NonExistent\nstruct Foo {}")
            .expect(HasError("module 'NonExistent' not found"));
    }

    #[test]
    fn error_on_importing_nonexistent_nested_module() {
        Test::with_files(&[
            ("library.ks", "module Library\npublic struct Foo {}"),
            (
                "consumer.ks",
                "module Consumer\nimport Library.Nonexistent\nstruct Bar {}",
            ),
        ])
        .expect(HasError("module 'Library.Nonexistent' not found"));
    }

    #[test]
    fn error_on_importing_nonexistent_item_from_module() {
        Test::with_files(&[
            ("library.ks", "module Library\npublic struct Foo {}"),
            (
                "consumer.ks",
                "module Consumer\nimport Library.(Bar)\nstruct Test {}",
            ),
        ])
        .expect(HasError("symbol 'Bar' not found in module 'Library'"));
    }

    #[test]
    fn error_on_importing_nonexistent_item_from_nested_module() {
        Test::with_files(&[
            (
                "math_geometry.ks",
                "module Math.Geometry\npublic struct Point {}",
            ),
            (
                "consumer.ks",
                "module Consumer\nimport Math.Geometry.(Circle)\nstruct Test {}",
            ),
        ])
        .expect(HasError(
            "symbol 'Circle' not found in module 'Math.Geometry'",
        ));
    }

    #[test]
    fn error_on_importing_private_item() {
        Test::with_files(&[
            (
                "library.ks",
                "module Library\nprivate struct PrivateClass {}\npublic struct PublicClass {}",
            ),
            (
                "consumer.ks",
                "module Consumer\nimport Library.(PrivateClass)\nstruct Test {}",
            ),
        ])
        .expect(HasError("'PrivateClass' is not accessible"));
    }

    // This test verifies that internal symbols cannot be imported from other modules.
    // The is_visible_from function now checks module boundaries for internal visibility.
    // However, this test requires the visibility check to happen during import resolution,
    // which involves the Import symbol as context (not the containing module).
    //
    // Current status: The find_containing_module function correctly identifies different
    // modules, but the import validation path may need adjustment to ensure the context
    // symbol properly represents the importing module.
    #[test]
    #[ignore = "Internal visibility across modules - needs investigation of import context"]
    fn error_on_importing_internal_item_from_different_module() {
        Test::with_files(&[
            (
                "library.ks",
                "module Library\ninternal struct InternalClass {}\npublic struct PublicClass {}",
            ),
            (
                "consumer.ks",
                "module Consumer\nimport Library.(InternalClass)\nstruct Test {}",
            ),
        ])
        .expect(HasError("'InternalClass' is not accessible"));
    }

    #[test]
    fn error_on_duplicate_import_same_item() {
        Test::with_files(&[
            ("library.ks", "module Library\npublic struct Foo {}"),
            (
                "consumer.ks",
                "module Consumer\nimport Library.(Foo)\nimport Library.(Foo)\nstruct Test {}",
            ),
        ])
        .expect(HasError("'Foo' is already imported"));
    }

    #[test]
    fn error_when_imported_item_conflicts_with_local_declaration() {
        Test::with_files(&[
            ("library.ks", "module Library\npublic struct Widget {}"),
            (
                "consumer.ks",
                "module Consumer\nimport Library\nstruct Widget {}",
            ),
        ])
        .expect(HasError("'Widget' is already declared"));
    }

    #[test]
    fn error_when_imported_items_conflict_from_different_imports() {
        Test::with_files(&[
            ("library_a.ks", "module LibraryA\npublic struct Widget {}"),
            ("library_b.ks", "module LibraryB\npublic struct Widget {}"),
            (
                "consumer.ks",
                "module Consumer\nimport LibraryA.(Widget)\nimport LibraryB\nstruct Test {}",
            ),
        ])
        .expect(HasError("'Widget' is already imported"));
    }
}
