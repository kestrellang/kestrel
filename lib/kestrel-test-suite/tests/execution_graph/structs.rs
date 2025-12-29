//! Struct MIR tests.
//!
//! Tests for struct lowering including:
//! - Struct definitions
//! - Struct construction
//! - Field access
//! - Methods (instance, static, mutating)
//! - Initializers
//! - Nested structs

use kestrel_test_suite::mir::*;
use kestrel_test_suite::*;

// ============================================================================
// STRUCT DEFINITIONS
// ============================================================================

mod struct_definitions {
    use super::*;

    #[test]
    fn simple_struct() {
        Test::new(
            r#"
            module Main

            struct Point {
                let x: Int
                let y: Int
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_struct("Main.Point")
                .has_field("x", MirTy::I64)
                .has_field("y", MirTy::I64)
                .has_field_count(2),
        );
    }

    #[test]
    fn struct_with_different_types() {
        Test::new(
            r#"
            module Main

            struct Person {
                let name: String
                let age: Int
                let active: Bool
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_struct("Main.Person")
                .has_field("name", MirTy::Str)
                .has_field("age", MirTy::I64)
                .has_field("active", MirTy::Bool)
                .has_field_count(3),
        );
    }

    #[test]
    fn struct_with_mutable_field() {
        // Based on tmp/08_mutating.ks
        Test::new(
            r#"
            module Main

            struct Counter {
                var count: Int
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_struct("Main.Counter")
                .has_field("count", MirTy::I64)
                .has_field_count(1),
        );
    }

    #[test]
    fn empty_struct() {
        Test::new(
            r#"
            module Main

            struct Empty { }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_struct("Main.Empty").has_field_count(0));
    }
}

// ============================================================================
// STRUCT CONSTRUCTION
// ============================================================================

mod struct_construction {
    use super::*;

    #[test]
    fn construct_simple_struct() {
        // Based on tmp/03_structs.ks
        Test::new(
            r#"
            module Main

            struct Point {
                let x: Int
                let y: Int
            }

            func makePoint() -> Point {
                Point(x: 3, y: 4)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.makePoint")
                .returns(MirTy::named("Main.Point"))
                .any_block(|b| {
                    b.has_statement(StatementPattern::Construct {
                        ty: "Main.Point".to_string(),
                    })
                }),
        );
    }

    #[test]
    fn construct_and_use_struct() {
        Test::new(
            r#"
            module Main

            struct Point {
                let x: Int
                let y: Int
            }

            func main() -> Int {
                let p = Point(x: 3, y: 4);
                p.x + p.y
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.main")
                .returns(MirTy::I64)
                .has_local("p", MirTy::named("Main.Point")),
        );
    }

    #[test]
    fn construct_nested_struct() {
        // Based on tmp/51_deeply_nested_struct.ks
        Test::new(
            r#"
            module Main

            struct Inner {
                let value: Int
            }

            struct Outer {
                let inner: Inner
            }

            func main() -> Outer {
                let i = Inner(value: 42);
                Outer(inner: i)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.main")
                .returns(MirTy::named("Main.Outer"))
                .has_local("i", MirTy::named("Main.Inner")),
        );
    }
}

// ============================================================================
// FIELD ACCESS
// ============================================================================

mod field_access {
    use super::*;

    #[test]
    fn simple_field_access() {
        Test::new(
            r#"
            module Main

            struct Point {
                let x: Int
                let y: Int
            }

            func getX(p: Point) -> Int {
                p.x
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.getX")
                .returns(MirTy::I64)
                .has_param("p", MirTy::named("Main.Point")),
        );
    }

    #[test]
    fn deeply_nested_field_access() {
        // Based on tmp/51_deeply_nested_struct.ks
        Test::new(
            r#"
            module Main

            struct Inner {
                let value: Int
            }

            struct Middle {
                let inner: Inner
            }

            struct Outer {
                let middle: Middle
            }

            func getValue(o: Outer) -> Int {
                o.middle.inner.value
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.getValue")
                .returns(MirTy::I64)
                .has_param("o", MirTy::named("Main.Outer")),
        );
    }

    #[test]
    fn four_level_nesting() {
        // Based on tmp/51_deeply_nested_struct.ks
        Test::new(
            r#"
            module Main

            struct Inner { let value: Int }
            struct Middle { let inner: Inner }
            struct Outer { let middle: Middle }
            struct Top { let outer: Outer }

            func getValue(t: Top) -> Int {
                t.outer.middle.inner.value
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::struct_count(4))
        .expect(
            Mir::mir_function("Main.getValue")
                .returns(MirTy::I64)
                .has_param("t", MirTy::named("Main.Top")),
        );
    }
}

// ============================================================================
// INSTANCE METHODS
// ============================================================================

mod instance_methods {
    use super::*;

    #[test]
    fn simple_method() {
        // Based on tmp/03_structs.ks
        Test::new(
            r#"
            module Main

            struct Point {
                let x: Int
                let y: Int
                
                func distanceSquared() -> Int {
                    self.x * self.x + self.y * self.y
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.Point.distanceSquared")
                .returns(MirTy::I64)
                .has_param("self", MirTy::ref_(MirTy::named("Main.Point")))
                .any_block(|b| b.has_statement(StatementPattern::BinOp(BinOp::MulSigned))),
        );
    }

    #[test]
    fn method_with_parameters() {
        Test::new(
            r#"
            module Main

            struct Point {
                let x: Int
                let y: Int
                
                func add(dx: Int, dy: Int) -> Point {
                    Point(x: self.x + dx, y: self.y + dy)
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.Point.add")
                .returns(MirTy::named("Main.Point"))
                .has_param("self", MirTy::ref_(MirTy::named("Main.Point")))
                .has_param("dx", MirTy::I64)
                .has_param("dy", MirTy::I64),
        );
    }

    #[test]
    fn multiple_methods() {
        // Based on tmp/03_structs.ks
        Test::new(
            r#"
            module Main

            struct Rectangle {
                let width: Int
                let height: Int
                
                func area() -> Int {
                    self.width * self.height
                }
                
                func perimeter() -> Int {
                    2 * (self.width + self.height)
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_function("Main.Rectangle.area").returns(MirTy::I64))
        .expect(Mir::mir_function("Main.Rectangle.perimeter").returns(MirTy::I64));
    }

    #[test]
    fn method_call() {
        Test::new(
            r#"
            module Main

            struct Point {
                let x: Int
                let y: Int
                
                func sum() -> Int {
                    self.x + self.y
                }
            }

            func main() -> Int {
                let p = Point(x: 3, y: 4);
                p.sum()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_function("Main.main").calls("Main.Point.sum"));
    }
}

// ============================================================================
// MUTATING METHODS
// ============================================================================

mod mutating_methods {
    use super::*;

    #[test]
    fn mutating_method_definition() {
        // Based on tmp/08_mutating.ks
        Test::new(
            r#"
            module Main

            struct Counter {
                var count: Int
                
                mutating func increment() {
                    self.count = self.count + 1;
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.Counter.increment")
                .returns(MirTy::Unit)
                .has_param("self", MirTy::ref_mut(MirTy::named("Main.Counter"))),
        );
    }

    #[test]
    fn mutating_method_with_param() {
        // Based on tmp/08_mutating.ks
        Test::new(
            r#"
            module Main

            struct Counter {
                var count: Int
                
                mutating func add(n: Int) {
                    self.count = self.count + n;
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.Counter.add")
                .returns(MirTy::Unit)
                .has_param("self", MirTy::ref_mut(MirTy::named("Main.Counter")))
                .has_param("n", MirTy::I64),
        );
    }

    #[test]
    fn mutating_method_call() {
        Test::new(
            r#"
            module Main

            struct Counter {
                var count: Int
                
                mutating func increment() {
                    self.count = self.count + 1;
                }

                func get() -> Int {
                    self.count
                }
            }

            func main() -> Int {
                var c = Counter(count: 0);
                c.increment();
                c.increment();
                c.get()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.main")
                .calls("Main.Counter.increment")
                .calls("Main.Counter.get"),
        );
    }
}

// ============================================================================
// INITIALIZERS
// ============================================================================

mod initializers {
    use super::*;

    #[test]
    fn simple_init() {
        // Based on tmp/08_mutating.ks
        // Note: init methods take &var self and return ()
        Test::new(
            r#"
            module Main

            struct Counter {
                var count: Int
                
                init() {
                    self.count = 0;
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.Counter.init")
                .returns(MirTy::Unit)
                .has_param("self", MirTy::ref_mut(MirTy::named("Main.Counter"))),
        );
    }

    #[test]
    fn init_with_parameter() {
        // Based on tmp/08_mutating.ks
        // Note: init methods take &var self and return ()
        Test::new(
            r#"
            module Main

            struct Counter {
                var count: Int
                
                init(start: Int) {
                    self.count = start;
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.Counter.init")
                .returns(MirTy::Unit)
                .has_param("self", MirTy::ref_mut(MirTy::named("Main.Counter")))
                .has_param("start", MirTy::I64),
        );
    }

    #[test]
    fn calling_init() {
        Test::new(
            r#"
            module Main

            struct Counter {
                var count: Int
                
                init(start: Int) {
                    self.count = start;
                }

                func get() -> Int {
                    self.count
                }
            }

            func main() -> Int {
                let c = Counter(start: 42);
                c.get()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.main")
                .calls("Main.Counter.init")
                .calls("Main.Counter.get"),
        );
    }
}

// ============================================================================
// STATIC METHODS
// ============================================================================

mod static_methods {
    use super::*;

    #[test]
    fn static_method_definition() {
        Test::new(
            r#"
            module Main

            struct Math {
                static func add(a: Int, b: Int) -> Int {
                    a + b
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.Math.add")
                .returns(MirTy::I64)
                .has_param("a", MirTy::I64)
                .has_param("b", MirTy::I64)
                .has_param_count(2), // No self parameter
        );
    }

    #[test]
    fn static_method_call() {
        Test::new(
            r#"
            module Main

            struct Math {
                static func add(a: Int, b: Int) -> Int {
                    a + b
                }
            }

            func main() -> Int {
                Math.add(3, 4)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_function("Main.main").calls("Main.Math.add"));
    }

    #[test]
    fn static_factory_method() {
        Test::new(
            r#"
            module Main

            struct Point {
                let x: Int
                let y: Int
                
                static func origin() -> Point {
                    Point(x: 0, y: 0)
                }
            }

            func main() -> Point {
                Point.origin()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.Point.origin")
                .returns(MirTy::named("Main.Point"))
                .has_param_count(0),
        )
        .expect(Mir::mir_function("Main.main").calls("Main.Point.origin"));
    }
}

// ============================================================================
// CHAINED METHOD CALLS
// ============================================================================

mod chained_methods {
    use super::*;

    #[test]
    fn chained_method_calls() {
        // Based on tmp/28_chained_methods.ks
        Test::new(
            r#"
            module Main

            struct Builder {
                let value: Int
                
                func add(n: Int) -> Builder {
                    Builder(value: self.value + n)
                }
                
                func multiply(n: Int) -> Builder {
                    Builder(value: self.value * n)
                }
                
                func build() -> Int {
                    self.value
                }
            }

            func main() -> Int {
                Builder(value: 0).add(5).multiply(3).build()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.main")
                .returns(MirTy::I64)
                .calls("Main.Builder.add")
                .calls("Main.Builder.multiply")
                .calls("Main.Builder.build"),
        );
    }
}

// ============================================================================
// STRUCTS WITH STRUCT FIELDS
// ============================================================================

mod struct_with_struct_fields {
    use super::*;

    #[test]
    fn struct_containing_struct() {
        Test::new(
            r#"
            module Main

            struct Point {
                let x: Int
                let y: Int
            }

            struct Rectangle {
                let origin: Point
                let width: Int
                let height: Int
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_struct("Main.Rectangle")
                .has_field("origin", MirTy::named("Main.Point"))
                .has_field("width", MirTy::I64)
                .has_field("height", MirTy::I64)
                .has_field_count(3),
        );
    }

    #[test]
    fn accessing_nested_struct_field() {
        Test::new(
            r#"
            module Main

            struct Point {
                let x: Int
                let y: Int
            }

            struct Rectangle {
                let origin: Point
                let width: Int
                let height: Int
            }

            func getOriginX(r: Rectangle) -> Int {
                r.origin.x
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.getOriginX")
                .returns(MirTy::I64)
                .has_param("r", MirTy::named("Main.Rectangle")),
        );
    }
}
