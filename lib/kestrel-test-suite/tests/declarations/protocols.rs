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
                func hash() -> lang.i64
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
                func lessThan(other: Self) -> lang.i1
                func greaterThan(other: Self) -> lang.i1
                func equals(other: Self) -> lang.i1
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
                func area() -> lang.i64
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
                func lessThan(other: lang.i64) -> lang.i1
                func equals(other: lang.i64) -> lang.i1
            }
            struct Number: Comparable {
                func lessThan(other: lang.i64) -> lang.i1 { true }
                func equals(other: lang.i64) -> lang.i1 { false }
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
                func lessThan(other: lang.i64) -> lang.i1
                func equals(other: lang.i64) -> lang.i1
            }
            struct Number: Comparable {
                func lessThan(other: lang.i64) -> lang.i1 { }
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
                func area() -> lang.i64
            }
            struct Circle: Drawable, Shape {
                func draw() { }
                func area() -> lang.i64 { 42 }
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
                func area() -> lang.i64
            }
            struct Circle: Shape {
                func area() -> lang.i64 { }
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
                func hash() -> lang.i64
            }
            struct Point: Hashable {
                func hash() -> lang.str { }
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
                func compare(other: lang.i64) -> lang.i1
            }
            struct Number: Comparable {
                func compare() -> lang.i1 { }
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
                func greet(with name: lang.str)
            }
            struct Person: Greetable {
                func greet(with name: lang.str) { }
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
                func greet(with name: lang.str)
            }
            struct Person: Greetable {
                func greet(using name: lang.str) { }
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

mod type_directed_conformance {
    use super::*;

    /// When a type has multiple initializers with the same label but different parameter types
    /// (from implementing multiple instantiations of a generic protocol), the compiler should
    /// select the correct one based on the argument type.
    #[test]
    fn initializer_selected_by_argument_type() {
        Test::new(
            r#"module Test

            public struct Wrapper8 {
                var raw: lang.i8
                public init(raw: lang.i8) { self.raw = raw }
            }

            public struct Wrapper32 {
                var raw: lang.i32
                public init(raw: lang.i32) { self.raw = raw }
            }

            // Target struct with differently-labeled inits
            public struct Target {
                var value: lang.i64

                public init(from8 value: Wrapper8) {
                    self.value = lang.cast_i8_i64(value.raw)
                }

                public init(from32 value: Wrapper32) {
                    self.value = lang.cast_i32_i64(value.raw)
                }
            }

            public func test() {
                let w8 = Wrapper8(lang.cast_i64_i8(1));
                let w32 = Wrapper32(lang.cast_i64_i32(42));

                // These should work - different labels select correct init
                let t1 = Target(from8: w8);
                let t2 = Target(from32: w32);
            }
        "#,
        )
        .expect(Compiles);
    }

    /// When a struct has multiple initializers with different labels but multiple candidates
    /// match by arity, the argument type determines which is called.
    #[test]
    fn type_directed_selection_with_different_labels() {
        Test::new(
            r#"module Test

            public struct Small {
                var x: lang.i8
                public init() { self.x = lang.cast_i64_i8(0) }
            }

            public struct Large {
                var x: lang.i32
                public init() { self.x = lang.cast_i64_i32(0) }
            }

            public struct Target {
                var value: lang.i64

                public init(fromSmall other: Small) {
                    self.value = lang.cast_i8_i64(other.x)
                }

                public init(fromLarge other: Large) {
                    self.value = lang.cast_i32_i64(other.x)
                }
            }

            public func test() {
                let s = Small();
                let l = Large();

                // Different labels - type-directed selection validates correct init is called
                let t1 = Target(fromSmall: s);
                let t2 = Target(fromLarge: l);
            }
        "#,
        )
        .expect(Compiles);
    }

    /// When a struct has multiple initializers with the same label implementing
    /// different protocol conformances, the argument type determines which is called.
    #[test]
    fn protocol_based_init_overloads() {
        Test::new(
            r#"module Test

            public protocol Convertible[T] {
                init(from other: T)
            }

            public struct Small {
                var x: lang.i8
                public init() { self.x = lang.cast_i64_i8(0) }
            }

            public struct Large {
                var x: lang.i32
                public init() { self.x = lang.cast_i64_i32(0) }
            }

            public struct Target: Convertible[Small], Convertible[Large] {
                var value: lang.i64

                public init(from other: Small) {
                    self.value = lang.cast_i8_i64(other.x)
                }

                public init(from other: Large) {
                    self.value = lang.cast_i32_i64(other.x)
                }
            }

            public func test() {
                let s = Small();
                let l = Large();

                // Type-directed conformance: selects init based on argument type
                let t1 = Target(from: s);  // Should call init(from: Small)
                let t2 = Target(from: l);  // Should call init(from: Large)
            }
        "#,
        )
        .expect(Compiles);
    }

    /// Test that method calls also use type-directed selection when multiple
    /// methods match by label/arity but differ in parameter types.
    #[test]
    fn method_call_type_directed_selection() {
        Test::new(
            r#"module Test

            public struct SmallValue {
                var x: lang.i8
                public init() { self.x = lang.cast_i64_i8(5) }
            }

            public struct LargeValue {
                var x: lang.i32
                public init() { self.x = lang.cast_i64_i32(100) }
            }

            public struct Processor {
                var result: lang.i64

                public init() { self.result = 0 }

                public func processSmall(value: SmallValue) -> lang.i64 {
                    lang.cast_i8_i64(value.x)
                }

                public func processLarge(value: LargeValue) -> lang.i64 {
                    lang.cast_i32_i64(value.x)
                }
            }

            public func test() {
                let p = Processor();
                let s = SmallValue();
                let l = LargeValue();

                // Method calls - function params don't have external labels by default
                let r1 = p.processSmall(s);
                let r2 = p.processLarge(l);
            }
        "#,
        )
        .expect(Compiles);
    }
}
