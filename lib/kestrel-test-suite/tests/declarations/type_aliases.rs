use kestrel_test_suite::*;

mod basic {
    use super::*;

    #[test]
    fn simple_type_alias() {
        Test::new("module Test\ntype Simple = Int;")
            .expect(Compiles)
            .expect(Symbol::new("Simple").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn visibility_modifiers() {
        Test::new(
            r#"module Test
            public type PublicAlias = String;
            internal type InternalAlias = Float;
            fileprivate type FilePrivateAlias = Int;
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("PublicAlias")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(
            Symbol::new("InternalAlias")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::Visibility(Visibility::Internal)),
        )
        .expect(
            Symbol::new("FilePrivateAlias")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::Visibility(Visibility::Fileprivate)),
        );
    }

    #[test]
    fn multiple_type_aliases() {
        Test::new(
            r#"module Test
            type Result = Int;
            type Maybe = String;
            type List = Bool;
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Result").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("Maybe").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("List").is(SymbolKind::TypeAlias));
    }
}

mod target_types {
    use super::*;

    #[test]
    fn builtin_type_targets() {
        Test::new(
            r#"module Test
            type IntAlias = Int;
            type StringAlias = String;
            type BoolAlias = Bool;
            type FloatAlias = Float;
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("IntAlias").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("StringAlias").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("BoolAlias").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("FloatAlias").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn struct_type_targets() {
        Test::new(
            r#"module Test
            public struct Color {}
            struct Point {}
            type PointAlias = Point;
            public type ColorAlias = Color;
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("PointAlias").is(SymbolKind::TypeAlias))
        .expect(
            Symbol::new("ColorAlias")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::Visibility(Visibility::Public)),
        );
    }

    #[test]
    fn tuple_type_targets() {
        Test::new(
            r#"module Test
            type Pair = (Int, String);
            type Triple = (Int, String, Bool);
            type Single = (Float);
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Pair").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("Triple").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("Single").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn nested_tuple_types() {
        Test::new(
            r#"module Test
            type NestedTuple = ((Int, String), Bool);
            type ComplexNesting = (Int, (String, (Bool, Float)));
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("NestedTuple").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("ComplexNesting").is(SymbolKind::TypeAlias));
    }
}

mod realistic {
    use super::*;

    #[test]
    fn domain_type_aliases() {
        Test::new(
            r#"module Application.Types
            public type UserID = String;
            public type Email = String;
            public type PhoneNumber = String;
            public type Timestamp = Int;
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("UserID")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(
            Symbol::new("Email")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(
            Symbol::new("PhoneNumber")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(
            Symbol::new("Timestamp")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::Visibility(Visibility::Public)),
        );
    }

    #[test]
    fn collection_aliases() {
        // Define the types that we alias to (public to match alias visibility)
        Test::new(
            r#"module Test
            public struct Array {}
            public struct Dictionary {}
            struct Set {}
            public type UserList = Array;
            public type UserMap = Dictionary;
            type UserSet = Set;
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("UserList")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(
            Symbol::new("UserMap")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(Symbol::new("UserSet").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn mixed_visibility_aliases() {
        Test::new(
            r#"module Test
            public type PublicResult = Bool;
            private type PrivateResult = Int;
            internal type InternalResult = String;
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("PublicResult")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(
            Symbol::new("PrivateResult")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::Visibility(Visibility::Private)),
        )
        .expect(
            Symbol::new("InternalResult")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::Visibility(Visibility::Internal)),
        );
    }

    #[test]
    fn chained_aliases() {
        Test::new(
            r#"module Test
            type Base = Int;
            type Derived = Base;
            type Final = Derived;
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Base").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("Derived").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("Final").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn multiple_aliases_same_target() {
        Test::new(
            r#"module Test
            type Alias1 = Int;
            type Alias2 = Int;
            type Alias3 = Int;
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Alias1").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("Alias2").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("Alias3").is(SymbolKind::TypeAlias));
    }
}

mod multi_file {
    use super::*;

    #[test]
    fn type_alias_with_imports() {
        Test::with_files(&[
            (
                "collections.ks",
                "module System.Collections\npublic struct Array {}",
            ),
            (
                "graphics.ks",
                r#"module Graphics
                import System.Collections
                public struct RGB {}
                struct Point2D {}
                public type Color = RGB;
                type Position = Point2D;"#,
            ),
        ])
        .expect(Compiles)
        .expect(
            Symbol::new("Color")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(Symbol::new("Position").is(SymbolKind::TypeAlias));
    }
}

mod cycle_detection {
    use super::*;

    #[test]
    fn self_reference_cycle() {
        Test::new("module Test\ntype A = A;").expect(HasError("circular type alias"));
    }

    #[test]
    fn two_way_cycle() {
        Test::new(
            r#"module Test
            type A = B;
            type B = A;
        "#,
        )
        .expect(HasError("circular type alias"));
    }

    #[test]
    fn multi_way_cycles() {
        Test::new(
            r#"module Test
            type A = B;
            type B = C;
            type C = A;
        "#,
        )
        .expect(HasError("circular type alias"));
    }

    #[test]
    fn cycle_in_tuple_type() {
        Test::new(
            r#"module Test
            type A = (B, Int);
            type B = A;
        "#,
        )
        .expect(HasError("circular type alias"));
    }

    #[test]
    fn valid_chain_to_builtin() {
        Test::new(
            r#"module Test
            type A = B;
            type B = Int;
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("A").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("B").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn valid_longer_chain() {
        Test::new(
            r#"module Test
            type A = B;
            type B = C;
            type C = D;
            type D = Int;
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("A").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("B").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("C").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("D").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn independent_chains() {
        Test::new(
            r#"module Test
            type A = B;
            type B = Int;
            type X = Y;
            type Y = String;
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("A").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("B").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("X").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("Y").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn valid_tuple_with_alias_reference() {
        Test::new(
            r#"module Test
            type A = (Int, B);
            type B = String;
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("A").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("B").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn mixed_valid_and_cyclic() {
        Test::new(
            r#"module Test
            type Valid1 = Int;
            type Valid2 = String;
            type Cycle1 = Cycle2;
            type Cycle2 = Cycle1;
        "#,
        )
        .expect(HasError("circular type alias"));
    }
}

mod unresolved_types {
    use super::*;

    // Tests for type resolution in type aliases

    #[test]
    fn type_alias_to_unknown_type() {
        Test::new(
            r#"module Test
            type Foo = Unknown;
        "#,
        )
        .expect(HasError("cannot find type"));
    }

    #[test]
    fn type_alias_to_unknown_in_tuple() {
        Test::new(
            r#"module Test
            type Foo = (Int, Unknown, String);
        "#,
        )
        .expect(HasError("cannot find type"));
    }
}
