use kestrel_test_suite::*;

mod basic {
    use super::*;

    #[test]
    fn empty_struct() {
        Test::new("module Test\nstruct Foo {}")
            .expect(Compiles)
            .expect(Symbol::new("Foo").is(SymbolKind::Struct));
    }

    #[test]
    fn visibility_modifiers() {
        // Test all visibility modifiers: public, private, internal, fileprivate, and default
        Test::new(
            r#"module Test
            public struct Public {}
            private struct Private {}
            internal struct Internal {}
            fileprivate struct Fileprivate {}
            struct Default {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Public")
                .is(SymbolKind::Struct)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(
            Symbol::new("Private")
                .is(SymbolKind::Struct)
                .has(Behavior::Visibility(Visibility::Private)),
        )
        .expect(
            Symbol::new("Internal")
                .is(SymbolKind::Struct)
                .has(Behavior::Visibility(Visibility::Internal)),
        )
        .expect(
            Symbol::new("Fileprivate")
                .is(SymbolKind::Struct)
                .has(Behavior::Visibility(Visibility::Fileprivate)),
        )
        .expect(
            Symbol::new("Default")
                .is(SymbolKind::Struct)
                .has(Behavior::Visibility(Visibility::Internal)),
        );
    }

    #[test]
    fn multiple_structs() {
        Test::new(
            r#"module Test
            struct First {}
            struct Second {}
            struct Third {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("First").is(SymbolKind::Struct))
        .expect(Symbol::new("Second").is(SymbolKind::Struct))
        .expect(Symbol::new("Third").is(SymbolKind::Struct));
    }
}

mod nested {
    use super::*;

    #[test]
    fn nested_struct() {
        Test::new(
            r#"module Test
            struct Outer {
                struct Inner {}
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Outer").is(SymbolKind::Struct))
        .expect(Symbol::new("Outer.Inner").is(SymbolKind::Struct));
    }

    #[test]
    fn deeply_nested_structs() {
        Test::new(
            r#"module Test
            struct Level1 {
                struct Level2 {
                    struct Level3 {}
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Level1").is(SymbolKind::Struct))
        .expect(Symbol::new("Level1.Level2").is(SymbolKind::Struct))
        .expect(Symbol::new("Level1.Level2.Level3").is(SymbolKind::Struct));
    }

    #[test]
    fn nested_struct_with_field() {
        Test::new(
            r#"module Test
            struct Container {
                var nested: lang.i64
                struct Nested {}
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Container")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(1)),
        )
        .expect(Symbol::new("Container.Nested").is(SymbolKind::Struct))
        .expect(Symbol::new("Container.nested").is(SymbolKind::Field));
    }
}

mod initializers {
    use super::*;

    #[test]
    fn explicit_initializer_with_parameters() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64

                init(x: lang.i64, y: lang.i64) {
                    self.x = x;
                    self.y = y;
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn initializer_without_params() {
        Test::new(
            r#"module Test
            struct Counter {
                var count: lang.i64

                init() {
                    self.count = 0;
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn multiple_initializers() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64

                init(x: lang.i64, y: lang.i64) {
                    self.x = x;
                    self.y = y;
                }

                init() {
                    self.x = 0;
                    self.y = 0;
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn initializer_with_visibility() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64

                public init(x: lang.i64) {
                    self.x = x;
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn initializer_with_labeled_params() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64

                init(atX x: lang.i64, atY y: lang.i64) {
                    self.x = x;
                    self.y = y;
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod instantiation {
    use super::*;

    #[test]
    fn empty_struct_instantiation() {
        Test::new(
            r#"module Test
            struct Empty {}

            func makeEmpty() -> Empty {
                Empty()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Empty").is(SymbolKind::Struct))
        .expect(Symbol::new("makeEmpty").is(SymbolKind::Function));
    }

    #[test]
    fn implicit_memberwise_init() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }

            func makePoint() -> Point {
                Point(x: 10, y: 20)
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2)),
        );
    }

    #[test]
    fn explicit_init_instantiation() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64

                init(x x: lang.i64, y y: lang.i64) {
                    self.x = x;
                    self.y = y;
                }
            }

            func makePoint() -> Point {
                Point(x: 5, y: 10)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn explicit_init_with_labeled_params() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64

                init(atX x: lang.i64, atY y: lang.i64) {
                    self.x = x;
                    self.y = y;
                }
            }

            func makePoint() -> Point {
                Point(atX: 5, atY: 10)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn multiple_init_overloads() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64

                init(x: lang.i64, y: lang.i64) {
                    self.x = x;
                    self.y = y;
                }

                init() {
                    self.x = 0;
                    self.y = 0;
                }

                init(value: lang.i64) {
                    self.x = value;
                    self.y = value;
                }
            }

            func test() {
                let p1 = Point(1, 2);
                let p2 = Point();
                let p3 = Point(5);
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn instantiation_with_various_field_counts() {
        Test::new(
            r#"module Test
            struct Single {
                var value: lang.i64
            }

            struct Many {
                var a: lang.i64
                var b: lang.i64
                var c: lang.i64
                var d: lang.i64
                var e: lang.i64
            }

            func makeSingle() -> Single {
                Single(value: 42)
            }

            func makeMany() -> Many {
                Many(a: 1, b: 2, c: 3, d: 4, e: 5)
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Single")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(1)),
        )
        .expect(
            Symbol::new("Many")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(5)),
        );
    }

    #[test]
    fn nested_struct_instantiation() {
        Test::new(
            r#"module Test
            struct Inner {
                var value: lang.i64
            }

            struct Outer {
                var inner: Inner
            }

            func makeOuter() -> Outer {
                Outer(inner: Inner(value: 42))
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Inner")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(1)),
        )
        .expect(
            Symbol::new("Outer")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(1)),
        );
    }

    #[test]
    fn instantiation_in_variable_binding() {
        Test::new("module Test\nstruct Point { var x: lang.i64\n var y: lang.i64 }\nfunc makePoint() -> Point { Point(x: 1, y: 2) }")
        .expect(Compiles)
        .expect(Symbol::new("Point").is(SymbolKind::Struct).has(Behavior::FieldCount(2)));
    }

    #[test]
    fn instantiation_as_function_argument() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }

            func takePoint(p: Point) -> lang.i64 {
                42
            }

            func test() -> lang.i64 {
                takePoint(Point(x: 1, y: 2))
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2)),
        )
        .expect(
            Symbol::new("takePoint")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn instantiation_with_mixed_field_mutability() {
        Test::new(
            r#"module Test
            struct Immutable {
                let x: lang.i64
                let y: lang.i64
            }

            struct Mixed {
                let id: lang.i64
                var value: lang.i64
            }

            func makeImmutable() -> Immutable {
                Immutable(x: 1, y: 2)
            }

            func makeMixed() -> Mixed {
                Mixed(id: 1, value: 2)
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Immutable")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2)),
        )
        .expect(
            Symbol::new("Mixed")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2)),
        );
    }
}

mod instantiation_errors {
    use super::*;

    #[test]
    fn calling_function_with_wrong_labels() {
        Test::new(
            r#"module Test
            func notAStruct() -> lang.i64 {
                42
            }

            func test() -> lang.i64 {
                notAStruct(x: 1)
            }
        "#,
        )
        .expect(HasError("no matching overload"));
    }

    #[test]
    fn wrong_arity_too_few() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }

            func test() -> Point {
                Point(x: 1)
            }
        "#,
        )
        .expect(HasError("has 2 field(s)"));
    }

    #[test]
    fn wrong_arity_too_many() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }

            func test() -> Point {
                Point(x: 1, y: 2, z: 3)
            }
        "#,
        )
        .expect(HasError("has 2 field(s)"));
    }

    #[test]
    fn wrong_label_name() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }

            func test() -> Point {
                Point(a: 1, b: 2)
            }
        "#,
        )
        .expect(HasError("label"));
    }

    #[test]
    fn struct_instantiation_success_after_error() {
        // Verify that struct instantiation works correctly after testing error cases
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }

            func test() -> Point {
                Point(x: 1, y: 2)
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2)),
        );
    }

    #[test]
    fn explicit_init_call_succeeds() {
        // Verify that an explicit init can be called correctly
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64

                init(x x: lang.i64, y y: lang.i64) {
                    self.x = x;
                    self.y = y;
                }
            }

            func test() -> Point {
                Point(x: 1, y: 2)
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod initializer_edge_cases {
    use super::*;

    #[test]
    fn init_parameter_shadows_field_name() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64

                init(x: lang.i64, y: lang.i64) {
                    self.x = x;
                    self.y = y;
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn init_with_different_param_names() {
        // Test that init parameters can have different names from fields
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64

                init(xVal: lang.i64, yVal: lang.i64) {
                    self.x = xVal;
                    self.y = yVal;
                }
            }

            func test() -> Point {
                Point(1, 2)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn init_with_function_call_arguments() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64

                init(x: lang.i64, y: lang.i64) {
                    self.x = x;
                    self.y = y;
                }
            }

            func getInt() -> lang.i64 {
                42
            }

            func test() -> Point {
                Point(getInt(), getInt())
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn init_body_with_local_variables() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64

                init(value: lang.i64) {
                    let doubled: lang.i64 = value;
                    self.x = doubled;
                    self.y = doubled;
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_with_multiple_methods() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64

                init(x: lang.i64, y: lang.i64) {
                    self.x = x;
                    self.y = y;
                }

                func sum() -> lang.i64 {
                    self.x
                }

                func product() -> lang.i64 {
                    self.y
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn init_with_empty_body() {
        Test::new(
            r#"module Test
            struct Empty {
                init() {
                }
            }

            func test() -> Empty {
                Empty()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Empty").is(SymbolKind::Struct));
    }
}
