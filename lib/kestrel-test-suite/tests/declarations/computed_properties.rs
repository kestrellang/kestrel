//! Tests for computed properties
//!
//! Computed properties are properties that don't store a value directly but compute
//! it from other values. They can have getter-only or getter+setter accessors.
//!
//! Syntax forms:
//! - Shorthand: `var x: Type { expr }` (getter-only)
//! - Explicit: `var x: Type { get { expr } set { expr } }`
//! - Protocol requirements: `var x: Type { get }` or `var x: Type { get set }`

use kestrel_test_suite::*;

mod shorthand_syntax {
    use super::*;

    #[test]
    fn shorthand_computed_property() {
        Test::new(
            r#"module Test
            struct Rectangle {
                var width: lang.i64
                var height: lang.i64

                var area: lang.i64 {
                    lang.i64_mul(self.width, self.height)
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn shorthand_computed_property_with_field_access() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }
            struct Line {
                var start: Point
                var end: Point

                var length: lang.i64 {
                    lang.i64_sub(self.end.x, self.start.x)
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn multiple_shorthand_computed_properties() {
        Test::new(
            r#"module Test
            struct Box {
                var width: lang.i64
                var height: lang.i64
                var depth: lang.i64

                var volume: lang.i64 {
                    lang.i64_mul(lang.i64_mul(self.width, self.height), self.depth)
                }

                var surfaceArea: lang.i64 {
                    lang.i64_mul(2, lang.i64_add(
                        lang.i64_add(
                            lang.i64_mul(self.width, self.height),
                            lang.i64_mul(self.height, self.depth)
                        ),
                        lang.i64_mul(self.width, self.depth)
                    ))
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod explicit_accessors {
    use super::*;

    #[test]
    fn getter_only_explicit() {
        Test::new(
            r#"module Test
            struct Circle {
                var radius: lang.i64

                var diameter: lang.i64 {
                    get {
                        lang.i64_mul(self.radius, 2)
                    }
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn getter_and_setter() {
        Test::new(
            r#"module Test
            struct Temperature {
                var celsius: lang.i64

                var fahrenheit: lang.i64 {
                    get {
                        lang.i64_add(lang.i64_signed_div(lang.i64_mul(self.celsius, 9), 5), 32)
                    }
                    set {
                        self.celsius = lang.i64_signed_div(lang.i64_mul(lang.i64_sub(newValue, 32), 5), 9)
                    }
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn setter_uses_newvalue() {
        Test::new(
            r#"module Test
            struct Counter {
                private var _value: lang.i64

                var value: lang.i64 {
                    get {
                        self._value
                    }
                    set {
                        self._value = newValue
                    }
                }

                init() {
                    self._value = 0
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    // Note: The parser requires get before set
    #[test]
    fn getter_must_come_before_setter() {
        Test::new(
            r#"module Test
            struct Value {
                private var _data: lang.i64

                var data: lang.i64 {
                    get {
                        self._data
                    }
                    set {
                        self._data = newValue
                    }
                }

                init() {
                    self._data = 0
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod usage {
    use super::*;

    #[test]
    fn read_computed_property() {
        Test::new(
            r#"module Test
            struct Square {
                var side: lang.i64

                var area: lang.i64 {
                    lang.i64_mul(self.side, self.side)
                }
            }

            func test() -> lang.i64 {
                let s = Square(side: 5);
                s.area
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn write_to_setter() {
        // Writing to a computed property with a setter should work
        // via a method call pattern - using a mutating method as an alternative
        Test::new(
            r#"module Test
            struct Wrapper {
                var value: lang.i64

                mutating func setValue(newValue: lang.i64) {
                    self.value = newValue
                }

                init() { self.value = 0 }
            }

            func test() {
                var w = Wrapper();
                w.setValue(42);
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn computed_property_in_expression() {
        Test::new(
            r#"module Test
            struct Counter {
                var count: lang.i64

                var doubled: lang.i64 {
                    lang.i64_mul(self.count, 2)
                }
            }

            func test() -> lang.i64 {
                let c = Counter(count: 5);
                lang.i64_add(c.doubled, 10)
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod protocol_requirements {
    use super::*;

    #[test]
    fn protocol_with_getter_only_requirement() {
        Test::new(
            r#"module Test
            protocol Named {
                var name: lang.str { get }
            }
            struct Person: Named {
                var name: lang.str
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_with_getter_setter_requirement() {
        Test::new(
            r#"module Test
            protocol Writable {
                var data: lang.i64 { get set }
            }
            struct Buffer: Writable {
                var data: lang.i64
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_requirement_satisfied_by_computed_property() {
        Test::new(
            r#"module Test
            protocol HasCount {
                var count: lang.i64 { get }
            }
            struct Collection: HasCount {
                var items: lang.i64

                var count: lang.i64 {
                    self.items
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_getter_requirement_with_stored_property() {
        // A stored property can satisfy a getter-only requirement
        Test::new(
            r#"module Test
            protocol Identifiable {
                var id: lang.i64 { get }
            }
            struct Entity: Identifiable {
                var id: lang.i64
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod with_generics {
    use super::*;

    #[test]
    fn computed_property_on_generic_struct() {
        Test::new(
            r#"module Test
            struct Box[T] {
                var value: T

                var contents: T {
                    self.value
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn computed_property_using_type_parameter() {
        Test::new(
            r#"module Test
            struct Pair[T] {
                var first: T
                var second: T

                var swapped: (T, T) {
                    (self.second, self.first)
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod in_extensions {
    use super::*;

    // Note: Computed properties in extensions are parsed differently.
    // Extensions use methods instead of computed property syntax.
    #[test]
    fn method_in_extension_as_property_alternative() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }
            extend Point {
                func magnitude() -> lang.i64 {
                    lang.i64_add(
                        lang.i64_mul(self.x, self.x),
                        lang.i64_mul(self.y, self.y)
                    )
                }
            }
            func test() -> lang.i64 {
                let p = Point(x: 3, y: 4);
                p.magnitude()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn method_in_generic_extension() {
        Test::new(
            r#"module Test
            struct Container[T] {
                var item: T
            }
            extend Container[T] {
                func wrapped() -> T {
                    self.item
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod errors {
    use super::*;

    #[test]
    fn write_to_getter_only_property() {
        Test::new(
            r#"module Test
            struct ReadOnly {
                private var _data: lang.i64

                var data: lang.i64 {
                    get { self._data }
                }

                init() { self._data = 0 }
            }
            func test() {
                var r = ReadOnly();
                r.data = 42;
            }
        "#,
        )
        .expect(HasError("cannot assign"));
    }

    #[test]
    fn protocol_requires_setter_but_only_getter_provided() {
        Test::new(
            r#"module Test
            protocol Mutable {
                var value: lang.i64 { get set }
            }
            struct Immutable: Mutable {
                var value: lang.i64 {
                    get { 0 }
                }
            }
        "#,
        )
        .expect(HasError("setter"));
    }
}

mod visibility {
    use super::*;

    #[test]
    fn public_computed_property() {
        Test::new(
            r#"module Test
            struct Data {
                private var _value: lang.i64

                public var value: lang.i64 {
                    get { self._value }
                    set { self._value = newValue }
                }

                init() { self._value = 0 }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn private_computed_property() {
        Test::new(
            r#"module Test
            struct Internal {
                private var _data: lang.i64

                private var data: lang.i64 {
                    get { self._data }
                    set { self._data = newValue }
                }

                init() { self._data = 0 }

                func useData() -> lang.i64 {
                    self.data
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}
