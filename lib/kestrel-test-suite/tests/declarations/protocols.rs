use kestrel_test_suite::*;

mod basic {
    use super::*;

    #[test]
    fn empty_protocol() {
        Test::new("module Test\nprotocol Drawable { }")
            .expect(Compiles)
            .expect(
                Symbol::new("Drawable")
                    .is(SymbolKind::Protocol)
                    .has(Behavior::ChildCount(0)),
            );
    }

    #[test]
    fn public_protocol() {
        Test::new("module Test\npublic protocol Equatable { }")
            .expect(Compiles)
            .expect(
                Symbol::new("Equatable")
                    .is(SymbolKind::Protocol)
                    .has(Behavior::Visibility(Visibility::Public))
                    .has(Behavior::ChildCount(0)),
            );
    }

    #[test]
    fn protocol_with_single_method() {
        Test::new(
            r#"module Test
            protocol Hashable {
                func hash() -> Int
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Hashable")
                .is(SymbolKind::Protocol)
                .has(Behavior::ChildCount(1)),
        )
        .expect(
            Symbol::new("Hashable.hash")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(false)),
        );
    }

    #[test]
    fn protocol_with_multiple_methods() {
        Test::new(
            r#"module Test
            protocol Comparable {
                func lessThan(other: Self) -> Bool
                func greaterThan(other: Self) -> Bool
                func equals(other: Self) -> Bool
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Comparable")
                .is(SymbolKind::Protocol)
                .has(Behavior::ChildCount(3)),
        )
        .expect(
            Symbol::new("Comparable.lessThan")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        )
        .expect(
            Symbol::new("Comparable.greaterThan")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        )
        .expect(
            Symbol::new("Comparable.equals")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }
}

mod conformance {
    use super::*;

    #[test]
    fn struct_with_single_conformance() {
        Test::new(
            r#"module Test
            protocol Drawable { }
            struct Point: Drawable { }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Drawable").is(SymbolKind::Protocol))
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::ConformanceCount(1)),
        );
    }

    #[test]
    fn struct_with_multiple_conformances() {
        Test::new(
            r#"module Test
            protocol Drawable { }
            protocol Equatable { }
            struct Point: Drawable, Equatable { }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::ConformanceCount(2)),
        );
    }

    #[test]
    fn generic_struct_with_conformance() {
        Test::new(
            r#"module Test
            protocol Container[T] { }
            struct Box[T]: Container[T] { }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Box")
                .is(SymbolKind::Struct)
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::IsGeneric(true))
                .has(Behavior::ConformanceCount(1)),
        )
        .expect(
            Symbol::new("Container")
                .is(SymbolKind::Protocol)
                .has(Behavior::TypeParamCount(1)),
        );
    }

    #[test]
    fn generic_struct_with_conformance_and_where_clause() {
        Test::new(
            r#"module Test
            protocol Equatable { }
            protocol Container[T] { }
            struct Set[T]: Container[T] where T: Equatable { }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Set")
                .is(SymbolKind::Struct)
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::IsGeneric(true))
                .has(Behavior::ConformanceCount(1)),
        );
    }

    #[test]
    fn protocol_conformance_applies_default_type_arguments() {
        Test::new(
            r#"module Test
            protocol Multipliable[Rhs = Self] {
                func multiply(other: Rhs) -> Self
            }
            struct Box: Multipliable {
                init() { }
                func multiply(other: Box) -> Box { Box() }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod inheritance {
    use super::*;

    #[test]
    fn protocol_inherits_single_protocol() {
        Test::new(
            r#"module Test
            protocol Drawable { }
            protocol Shape: Drawable { }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Drawable").is(SymbolKind::Protocol))
        .expect(
            Symbol::new("Shape")
                .is(SymbolKind::Protocol)
                .has(Behavior::ConformanceCount(1)),
        );
    }

    #[test]
    fn protocol_inherits_multiple_protocols() {
        Test::new(
            r#"module Test
            protocol Drawable { }
            protocol Clickable { }
            protocol Widget: Drawable, Clickable { }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Widget")
                .is(SymbolKind::Protocol)
                .has(Behavior::ConformanceCount(2))
                .has(Behavior::ChildCount(0)),
        );
    }

    #[test]
    fn protocol_with_inherited_method_and_own_method() {
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
            }
            protocol Shape: Drawable {
                func area() -> Int
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Shape")
                .is(SymbolKind::Protocol)
                .has(Behavior::ConformanceCount(1))
                .has(Behavior::ChildCount(1)),
        )
        .expect(
            Symbol::new("Shape.area")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(false)),
        );
    }

    #[test]
    fn generic_protocol_with_inheritance() {
        Test::new(
            r#"module Test
            protocol Comparable { }
            protocol Sortable[T]: Comparable { }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Sortable")
                .is(SymbolKind::Protocol)
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::IsGeneric(true))
                .has(Behavior::ConformanceCount(1)),
        );
    }
}

mod validation {
    use super::*;

    #[test]
    fn struct_implements_required_method() {
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
            }
            struct Circle: Drawable {
                func draw() { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Circle")
                .is(SymbolKind::Struct)
                .has(Behavior::ConformanceCount(1))
                .has(Behavior::ChildCount(1)),
        )
        .expect(
            Symbol::new("Circle.draw")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        );
    }

    #[test]
    fn struct_missing_required_method() {
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
            }
            struct Circle: Drawable { }
        "#,
        )
        .expect(HasError("does not implement method 'draw'"));
    }

    #[test]
    fn struct_implements_all_protocol_methods() {
        Test::new(
            r#"module Test
            protocol Comparable {
                func lessThan(other: Int) -> Bool
                func equals(other: Int) -> Bool
            }
            struct Number: Comparable {
                func lessThan(other: Int) -> Bool { true }
                func equals(other: Int) -> Bool { false }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Number")
                .is(SymbolKind::Struct)
                .has(Behavior::ConformanceCount(1))
                .has(Behavior::ChildCount(2)),
        )
        .expect(
            Symbol::new("Number.lessThan")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        )
        .expect(
            Symbol::new("Number.equals")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn struct_missing_one_of_multiple_methods() {
        Test::new(
            r#"module Test
            protocol Comparable {
                func lessThan(other: Int) -> Bool
                func equals(other: Int) -> Bool
            }
            struct Number: Comparable {
                func lessThan(other: Int) -> Bool { }
            }
        "#,
        )
        .expect(HasError("does not implement method 'equals'"));
    }

    #[test]
    fn struct_implements_inherited_protocol_methods() {
        // When conforming to Shape (which inherits from Drawable),
        // must also explicitly conform to Drawable
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
            }
            protocol Shape: Drawable {
                func area() -> Int
            }
            struct Circle: Drawable, Shape {
                func draw() { }
                func area() -> Int { 42 }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Circle")
                .is(SymbolKind::Struct)
                .has(Behavior::ConformanceCount(2))
                .has(Behavior::ChildCount(2)),
        );
    }

    #[test]
    fn struct_missing_inherited_protocol_method() {
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
            }
            protocol Shape: Drawable {
                func area() -> Int
            }
            struct Circle: Shape {
                func area() -> Int { }
            }
        "#,
        )
        .expect(HasError("does not implement method 'draw'"));
    }

    #[test]
    fn struct_with_method_wrong_return_type() {
        Test::new(
            r#"module Test
            protocol Hashable {
                func hash() -> Int
            }
            struct Point: Hashable {
                func hash() -> String { }
            }
        "#,
        )
        .expect(HasError("method 'hash' has wrong return type"));
    }

    #[test]
    fn struct_with_method_wrong_parameter_count() {
        Test::new(
            r#"module Test
            protocol Comparable {
                func compare(other: Int) -> Bool
            }
            struct Number: Comparable {
                func compare() -> Bool { }
            }
        "#,
        )
        .expect(HasError("does not implement method 'compare'"));
    }

    #[test]
    fn struct_implements_multiple_protocol_conformances() {
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
            }
            protocol Clickable {
                func onClick()
            }
            struct Button: Drawable, Clickable {
                func draw() { }
                func onClick() { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Button")
                .is(SymbolKind::Struct)
                .has(Behavior::ConformanceCount(2))
                .has(Behavior::ChildCount(2)),
        )
        .expect(Symbol::new("Button.draw").is(SymbolKind::Function))
        .expect(Symbol::new("Button.onClick").is(SymbolKind::Function));
    }

    #[test]
    fn struct_missing_method_from_second_conformance() {
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
            }
            protocol Clickable {
                func onClick()
            }
            struct Button: Drawable, Clickable {
                func draw() { }
            }
        "#,
        )
        .expect(HasError("does not implement method 'onClick'"));
    }

    #[test]
    fn protocol_conformance_with_inherited_protocols() {
        // When conforming to B (which inherits from A), must also explicitly conform to A
        Test::new(
            r#"module Test
            protocol A { func a() }
            protocol B: A { func b() }
            struct S: A, B {
                func a() { }
                func b() { }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_missing_method_from_inherited_protocol() {
        // TODO: Check inherited protocols
        Test::new(
            r#"module Test
            protocol A { func a() }
            protocol B: A { func b() }
            struct S: B {
                func b() { }
            }
        "#,
        )
        .expect(HasError("does not implement method 'a'"));
    }

    #[test]
    fn empty_protocol_conformance_requires_no_methods() {
        Test::new(
            r#"module Test
            protocol Marker { }
            struct Point: Marker { }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::ConformanceCount(1))
                .has(Behavior::ChildCount(0)),
        );
    }

    #[test]
    fn struct_with_labeled_parameter_method() {
        Test::new(
            r#"module Test
            protocol Greetable {
                func greet(with name: String)
            }
            struct Person: Greetable {
                func greet(with name: String) { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Person")
                .is(SymbolKind::Struct)
                .has(Behavior::ConformanceCount(1)),
        )
        .expect(
            Symbol::new("Person.greet")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn struct_with_wrong_label_on_method() {
        Test::new(
            r#"module Test
            protocol Greetable {
                func greet(with name: String)
            }
            struct Person: Greetable {
                func greet(using name: String) { }
            }
        "#,
        )
        .expect(HasError("does not implement method 'greet'"));
    }
}

mod regression {
    use super::*;

    /// Regression test for: Generic init with where clause not supported
    /// Issue: Generic initializers with where clauses in protocols weren't getting their
    /// type parameters registered as child symbols during semantic analysis, causing
    /// "cannot find type 'I' in this scope" errors.
    #[test]
    fn generic_init_with_where_clause() {
        Test::new(
            r#"module Test
            public protocol Iterator {
                type Item
            }

            public protocol Collectable {
                type Item

                init[I](from iter: I) where I: Iterator, I.Item = Item
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Collectable").is(SymbolKind::Protocol));
    }

    /// Regression test for: Child protocol cannot redeclare parent's associated type
    /// Issue: When a protocol inherits from another protocol and redeclares an associated type
    /// with the same name, the compiler incorrectly treated this as a conflict error.
    /// This should be allowed - children can refine/override parent associated types.
    #[test]
    fn child_protocol_can_redeclare_parent_associated_type() {
        Test::new(
            r#"module Test
            public protocol _ExpressibleByArrayLiteral {
                type Element
            }

            public protocol ExpressibleByArrayLiteral: _ExpressibleByArrayLiteral {
                type Element
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("_ExpressibleByArrayLiteral")
                .is(SymbolKind::Protocol)
                .has(Behavior::ChildCount(1)),
        )
        .expect(
            Symbol::new("ExpressibleByArrayLiteral")
                .is(SymbolKind::Protocol)
                .has(Behavior::ChildCount(1))
                .has(Behavior::ConformanceCount(1)),
        );
    }

    /// Regression test for: Diamond inheritance should still error on conflicting associated types
    /// This test ensures that the fix for allowing child protocols to redeclare parent's associated
    /// types doesn't break the legitimate error case where two sibling protocols define the same
    /// associated type (diamond inheritance conflict).
    #[test]
    fn diamond_inheritance_associated_type_conflict() {
        Test::new(
            r#"module Test
            protocol A {
                type Element
            }

            protocol B {
                type Element
            }

            protocol C: A, B {
            }
        "#,
        )
        .expect(HasError("conflicting associated type 'Element'"));
    }

    /// Regression test for: Protocol extension default implementations not inherited
    /// Issue: When a protocol extends another protocol and provides a default implementation
    /// via `extend`, types conforming to the child protocol should automatically get the
    /// default implementation without having to implement it themselves.
    #[test]
    fn protocol_extension_default_implementation() {
        Test::new(
            r#"module Test
            public protocol Parent {
                func parentMethod() -> lang.i64
            }

            // Provide default implementation in an extension
            extend Parent {
                public func parentMethod() -> lang.i64 {
                    42
                }
            }

            public protocol Child: Parent {
                func childMethod() -> lang.i64
            }

            // MyStruct should only need to implement childMethod,
            // not parentMethod (it has a default implementation)
            public struct MyStruct: Child {
                public func childMethod() -> lang.i64 {
                    10
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("MyStruct").is(SymbolKind::Struct));
    }
}
