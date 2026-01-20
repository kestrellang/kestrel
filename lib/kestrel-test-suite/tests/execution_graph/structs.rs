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
                let x: lang.i64
                let y: lang.i64
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
                let name: lang.str                let age: lang.i64
                let active: lang.i1
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
                var count: lang.i64
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
                let x: lang.i64
                let y: lang.i64
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
                let x: lang.i64
                let y: lang.i64
            }

            func main() -> lang.i64 {
                let p = Point(x: 3, y: 4);
                lang.i64_add(p.x, p.y)
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
                let value: lang.i64
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
        // Note: Parameters default to borrow mode, so they have reference types
        Test::new(
            r#"
            module Main

            struct Point {
                let x: lang.i64
                let y: lang.i64
            }

            func getX(p: Point) -> lang.i64 {
                p.x
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.getX")
                .returns(MirTy::I64)
                .has_param("p", MirTy::ref_(MirTy::named("Main.Point"))),
        );
    }

    #[test]
    fn deeply_nested_field_access() {
        // Based on tmp/51_deeply_nested_struct.ks
        // Note: Parameters default to borrow mode
        Test::new(
            r#"
            module Main

            struct Inner {
                let value: lang.i64
            }

            struct Middle {
                let inner: Inner
            }

            struct Outer {
                let middle: Middle
            }

            func getValue(o: Outer) -> lang.i64 {
                o.middle.inner.value
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.getValue")
                .returns(MirTy::I64)
                .has_param("o", MirTy::ref_(MirTy::named("Main.Outer"))),
        );
    }

    #[test]
    fn four_level_nesting() {
        // Based on tmp/51_deeply_nested_struct.ks
        // Note: Parameters default to borrow mode
        Test::new(
            r#"
            module Main

            struct Inner { let value: lang.i64 }
            struct Middle { let inner: Inner }
            struct Outer { let middle: Middle }
            struct Top { let outer: Outer }

            func getValue(t: Top) -> lang.i64 {
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
                .has_param("t", MirTy::ref_(MirTy::named("Main.Top"))),
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
                let x: lang.i64
                let y: lang.i64
                
                func distanceSquared() -> lang.i64 {
                    lang.i64_add(lang.i64_mul(self.x, self.x), lang.i64_mul(self.y, self.y))
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
        // Note: Regular parameters default to borrow mode
        Test::new(
            r#"
            module Main

            struct Point {
                let x: lang.i64
                let y: lang.i64
                
                func add(dx: lang.i64, dy: lang.i64) -> Point {
                    Point(x: lang.i64_add(self.x, dx), y: lang.i64_add(self.y, dy))
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
                .has_param("dx", MirTy::ref_(MirTy::I64))
                .has_param("dy", MirTy::ref_(MirTy::I64)),
        );
    }

    #[test]
    fn multiple_methods() {
        // Based on tmp/03_structs.ks
        Test::new(
            r#"
            module Main

            struct Rectangle {
                let width: lang.i64
                let height: lang.i64
                
                func area() -> lang.i64 {
                    lang.i64_mul(self.width, self.height)
                }

                func perimeter() -> lang.i64 {
                    lang.i64_mul(2, lang.i64_add(self.width, self.height))
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
                let x: lang.i64
                let y: lang.i64
                
                func sum() -> lang.i64 {
                    lang.i64_add(self.x, self.y)
                }
            }

            func main() -> lang.i64 {
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
                var count: lang.i64
                
                mutating func increment() {
                    self.count = lang.i64_add(self.count, 1);
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
        // Note: Regular parameters default to borrow mode
        Test::new(
            r#"
            module Main

            struct Counter {
                var count: lang.i64
                
                mutating func add(n: lang.i64) {
                    self.count = lang.i64_add(self.count, n);
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
                .has_param("n", MirTy::ref_(MirTy::I64)),
        );
    }

    #[test]
    fn mutating_method_call() {
        Test::new(
            r#"
            module Main

            struct Counter {
                var count: lang.i64
                
                mutating func increment() {
                    self.count = lang.i64_add(self.count, 1);
                }

                func read() -> lang.i64 {
                    self.count
                }
            }

            func main() -> lang.i64 {
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
                var count: lang.i64
                
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
        // Note: Regular parameters default to borrow mode
        Test::new(
            r#"
            module Main

            struct Counter {
                var count: lang.i64
                
                init(start: lang.i64) {
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
                .has_param("start", MirTy::ref_(MirTy::I64)),
        );
    }

    #[test]
    fn calling_init() {
        Test::new(
            r#"
            module Main

            struct Counter {
                var count: lang.i64
                
                init(start: lang.i64) {
                    self.count = start;
                }

                func read() -> lang.i64 {
                    self.count
                }
            }

            func main() -> lang.i64 {
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
        // Note: Regular parameters default to borrow mode
        Test::new(
            r#"
            module Main

            struct Math {
                static func add(a: lang.i64, b: lang.i64) -> lang.i64 {
                    lang.i64_add(a, b)
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.Math.add")
                .returns(MirTy::I64)
                .has_param("a", MirTy::ref_(MirTy::I64))
                .has_param("b", MirTy::ref_(MirTy::I64))
                .has_param_count(2), // No self parameter
        );
    }

    #[test]
    fn static_method_call() {
        Test::new(
            r#"
            module Main

            struct Math {
                static func add(a: lang.i64, b: lang.i64) -> lang.i64 {
                    lang.i64_add(a, b)
                }
            }

            func main() -> lang.i64 {
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
                let x: lang.i64
                let y: lang.i64
                
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
                let value: lang.i64
                
                func add(n: lang.i64) -> Builder {
                    Builder(value: lang.i64_add(self.value, n))
                }

                func multiply(n: lang.i64) -> Builder {
                    Builder(value: lang.i64_mul(self.value, n))
                }
                
                func build() -> lang.i64 {
                    self.value
                }
            }

            func main() -> lang.i64 {
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
                let x: lang.i64
                let y: lang.i64
            }

            struct Rectangle {
                let origin: Point
                let width: lang.i64
                let height: lang.i64
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
        // Note: Parameters default to borrow mode
        Test::new(
            r#"
            module Main

            struct Point {
                let x: lang.i64
                let y: lang.i64
            }

            struct Rectangle {
                let origin: Point
                let width: lang.i64
                let height: lang.i64
            }

            func getOriginX(r: Rectangle) -> lang.i64 {
                r.origin.x
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.getOriginX")
                .returns(MirTy::I64)
                .has_param("r", MirTy::ref_(MirTy::named("Main.Rectangle"))),
        );
    }
}
