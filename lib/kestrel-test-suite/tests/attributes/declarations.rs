//! Attribute tests for each declaration type
//!
//! Systematic tests ensuring attributes work correctly on all 7 declaration types:
//! - Protocol declarations
//! - Struct declarations
//! - Enum declarations
//! - Function declarations
//! - Field declarations
//! - Initializer declarations
//! - Enum case declarations

use kestrel_test_suite::*;

// =============================================================================
// PROTOCOL DECLARATIONS
// =============================================================================

mod protocol_declarations {
    use super::*;

    #[test]
    fn simple_protocol_with_attribute() {
        Test::new(
            r#"module Test
            @dummy
            protocol Drawable {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Drawable")
                .is(SymbolKind::Protocol)
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn protocol_with_attribute_and_method() {
        Test::new(
            r#"module Test
            @dummy
            protocol Drawable {
                func draw()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Drawable").has(Behavior::HasAttribute("dummy")))
        .expect(Symbol::new("Drawable.draw").is(SymbolKind::Function));
    }

    #[test]
    fn protocol_with_attribute_and_associated_type() {
        Test::new(
            r#"module Test
            @dummy
            protocol Iterator {
                type Item;
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Iterator").has(Behavior::HasAttribute("dummy")));
    }

    #[test]
    fn protocol_inheriting_with_attribute() {
        Test::new(
            r#"module Test
            protocol Base {}
            
            @dummy
            protocol Derived: Base {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Derived")
                .has(Behavior::ConformanceCount(1))
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn generic_protocol_with_attribute() {
        Test::new(
            r#"module Test
            @dummy
            protocol Container[T] {
                func read() -> T
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Container")
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::HasAttribute("dummy")),
        );
    }
}

// =============================================================================
// STRUCT DECLARATIONS
// =============================================================================

mod struct_declarations {
    use super::*;

    #[test]
    fn simple_struct_with_attribute() {
        Test::new(
            r#"module Test
            @dummy
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2))
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn struct_with_attribute_and_methods() {
        Test::new(
            r#"module Test
            @dummy
            struct Point {
                var x: lang.i64
                var y: lang.i64
                
                func magnitude() -> lang.i64 {
                    lang.i64_add(self.x, self.y)
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Point").has(Behavior::HasAttribute("dummy")))
        .expect(Symbol::new("Point.magnitude").is(SymbolKind::Function));
    }

    #[test]
    fn struct_with_conformance_and_attribute() {
        Test::new(
            r#"module Test
            protocol Printable {}
            
            @dummy
            struct Point: Printable {
                var x: lang.i64
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point")
                .has(Behavior::ConformanceCount(1))
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn generic_struct_with_attribute() {
        Test::new(
            r#"module Test
            @dummy
            struct Box[T] {
                var value: T
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Box")
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn nested_struct_with_attribute() {
        Test::new(
            r#"module Test
            struct Outer {
                @dummy
                struct Inner {
                    var x: lang.i64
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Outer.Inner").has(Behavior::HasAttribute("dummy")));
    }
}

// =============================================================================
// ENUM DECLARATIONS
// =============================================================================

mod enum_declarations {
    use super::*;

    #[test]
    fn simple_enum_with_attribute() {
        Test::new(
            r#"module Test
            @dummy
            enum Color {
                case Red
                case Green
                case Blue
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Color")
                .is(SymbolKind::Enum)
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn enum_with_payloads_and_attribute() {
        Test::new(
            r#"module Test
            @dummy
            enum Result {
                case Success(value: lang.i64)
                case Failure(message: lang.str)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Result").has(Behavior::HasAttribute("dummy")));
    }

    #[test]
    fn indirect_enum_with_attribute() {
        Test::new(
            r#"module Test
            @dummy
            indirect enum Tree {
                case Leaf(value: lang.i64)
                case Node(left: Tree, right: Tree)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Tree").has(Behavior::HasAttribute("dummy")));
    }

    #[test]
    fn generic_enum_with_attribute() {
        Test::new(
            r#"module Test
            @dummy
            enum Option[T] {
                case Some(value: T)
                case None
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Option")
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn enum_with_conformance_and_attribute() {
        Test::new(
            r#"module Test
            protocol Printable {}
            
            @dummy
            enum Status: Printable {
                case Active
                case Inactive
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Status")
                .has(Behavior::ConformanceCount(1))
                .has(Behavior::HasAttribute("dummy")),
        );
    }
}

// =============================================================================
// FUNCTION DECLARATIONS
// =============================================================================

mod function_declarations {
    use super::*;

    #[test]
    fn simple_function_with_attribute() {
        Test::new(
            r#"module Test
            @dummy
            func greet() {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("greet")
                .is(SymbolKind::Function)
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn function_with_params_and_attribute() {
        Test::new(
            r#"module Test
            @dummy
            func add(a: lang.i64, b: lang.i64) -> lang.i64 {
                lang.i64_add(a, b)
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("add")
                .has(Behavior::ParameterCount(2))
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn generic_function_with_attribute() {
        Test::new(
            r#"module Test
            @dummy
            func identity[T](value: T) -> T {
                value
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("identity")
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn method_with_attribute() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                
                @dummy
                func getX() -> lang.i64 {
                    self.x
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point.getX")
                .has(Behavior::IsInstanceMethod(true))
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn static_method_with_attribute() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                
                @dummy
                static func origin() -> Point {
                    Point(x: 0)
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point.origin")
                .has(Behavior::IsStatic(true))
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn mutating_method_with_attribute() {
        Test::new(
            r#"module Test
            struct Counter {
                var count: lang.i64
                
                @dummy
                mutating func increment() {
                    self.count = lang.i64_add(self.count, 1);
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Counter.increment").has(Behavior::HasAttribute("dummy")));
    }

    #[test]
    fn consuming_method_with_attribute() {
        Test::new(
            r#"module Test
            struct Resource {
                var data: lang.i64
                
                @dummy
                consuming func take() -> lang.i64 {
                    self.data
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Resource.take").has(Behavior::HasAttribute("dummy")));
    }

    #[test]
    fn protocol_method_with_attribute() {
        Test::new(
            r#"module Test
            protocol Drawable {
                @dummy
                func draw()
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Drawable.draw")
                .has(Behavior::HasBody(false))
                .has(Behavior::HasAttribute("dummy")),
        );
    }
}

// =============================================================================
// FIELD DECLARATIONS
// =============================================================================

mod field_declarations {
    use super::*;

    #[test]
    fn var_field_with_attribute() {
        Test::new(
            r#"module Test
            struct Point {
                @dummy
                var x: lang.i64
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("x")
                .is(SymbolKind::Field)
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn let_field_with_attribute() {
        Test::new(
            r#"module Test
            struct Point {
                @dummy
                let x: lang.i64
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("x").has(Behavior::HasAttribute("dummy")));
    }

    #[test]
    fn multiple_fields_with_attributes() {
        Test::new(
            r#"module Test
            struct Point {
                @dummy
                var x: lang.i64
                @dummy
                var y: lang.i64
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("x").has(Behavior::HasAttribute("dummy")))
        .expect(Symbol::new("y").has(Behavior::HasAttribute("dummy")));
    }

    #[test]
    fn field_with_attribute_and_args() {
        Test::new(
            r#"module Test
            struct Config {
                @dummy(default: 42)
                var timeout: lang.i64
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("timeout")
                .has(Behavior::HasAttribute("dummy"))
                .has(Behavior::AttributeArgCount("dummy", 1)),
        );
    }

    #[test]
    fn public_field_with_attribute() {
        Test::new(
            r#"module Test
            struct Point {
                @dummy
                public var x: lang.i64
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("x")
                .has(Behavior::Visibility(Visibility::Public))
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn static_field_with_attribute() {
        Test::new(
            r#"module Test
            struct Constants {
                @dummy
                static let pi: lang.f64
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Constants.pi").has(Behavior::HasAttribute("dummy")));
    }
}

// =============================================================================
// INITIALIZER DECLARATIONS
// =============================================================================

mod initializer_declarations {
    use super::*;

    #[test]
    fn simple_initializer_with_attribute() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                
                @dummy
                init(x: lang.i64) {
                    self.x = x;
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point.init")
                .is(SymbolKind::Initializer)
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn initializer_with_multiple_params_and_attribute() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64
                
                @dummy
                init(x: lang.i64, y: lang.i64) {
                    self.x = x;
                    self.y = y;
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point.init")
                .has(Behavior::ParameterCount(2))
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn public_initializer_with_attribute() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                
                @dummy
                public init(x: lang.i64) {
                    self.x = x;
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point.init")
                .has(Behavior::Visibility(Visibility::Public))
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn protocol_initializer_with_attribute() {
        Test::new(
            r#"module Test
            protocol Buildable {
                @dummy
                init()
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Buildable.init")
                .is(SymbolKind::Initializer)
                .has(Behavior::HasAttribute("dummy")),
        );
    }
}

// =============================================================================
// ENUM CASE DECLARATIONS
// =============================================================================

mod enum_case_declarations {
    use super::*;

    #[test]
    fn simple_case_with_attribute() {
        Test::new(
            r#"module Test
            enum Status {
                @dummy
                case Active
                case Inactive
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Active")
                .is(SymbolKind::EnumCase)
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn case_with_payload_and_attribute() {
        Test::new(
            r#"module Test
            enum Result {
                @dummy
                case Success(value: lang.i64)
                case Failure(message: lang.str)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Success").has(Behavior::HasAttribute("dummy")));
    }

    #[test]
    fn multiple_cases_with_attributes() {
        Test::new(
            r#"module Test
            enum Status {
                @dummy
                case Active
                @dummy
                case Pending
                case Inactive
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Active").has(Behavior::HasAttribute("dummy")))
        .expect(Symbol::new("Pending").has(Behavior::HasAttribute("dummy")))
        .expect(Symbol::new("Inactive").has(Behavior::AttributeCount(0)));
    }

    #[test]
    fn case_with_attribute_and_args() {
        Test::new(
            r#"module Test
            enum Priority {
                @dummy(level: 1)
                case High
                @dummy(level: 2)
                case Medium
                @dummy(level: 3)
                case Low
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("High").has(Behavior::AttributeArgCount("dummy", 1)))
        .expect(Symbol::new("Medium").has(Behavior::AttributeArgCount("dummy", 1)))
        .expect(Symbol::new("Low").has(Behavior::AttributeArgCount("dummy", 1)));
    }
}
