use kestrel_test_suite::*;

mod basic {
    use super::*;

    #[test]
    fn struct_method_implements_protocol_method() {
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
            Symbol::new("Circle.draw")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Drawable", "draw")),
        );
    }

    #[test]
    fn struct_method_with_parameters() {
        Test::new(
            r#"module Test
            protocol Comparable {
                func compare(other: lang.i64) -> lang.i1
            }
            struct Number: Comparable {
                func compare(other: lang.i64) -> lang.i1 { true }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Number.compare")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Comparable", "compare")),
        );
    }

    #[test]
    fn struct_method_with_labeled_parameter() {
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
            Symbol::new("Person.greet")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Greetable", "greet")),
        );
    }

    #[test]
    fn struct_method_with_return_type() {
        Test::new(
            r#"module Test
            protocol Hashable {
                func hash() -> lang.i64
            }
            struct Point: Hashable {
                func hash() -> lang.i64 { 42 }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point.hash")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Hashable", "hash")),
        );
    }

    #[test]
    fn multiple_methods_implementing_different_protocol_methods() {
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
            Symbol::new("Number.lessThan")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Comparable", "lessThan")),
        )
        .expect(
            Symbol::new("Number.equals")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Comparable", "equals")),
        );
    }

    #[test]
    fn struct_method_not_implementing_protocol() {
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
            }
            struct Circle: Drawable {
                func draw() { }
                func rotate() { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Circle.draw")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Drawable", "draw")),
        )
        .expect(
            Symbol::new("Circle.rotate")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocolNone),
        );
    }
}

mod inheritance {
    use super::*;

    #[test]
    fn struct_implements_inherited_protocol_method() {
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
            Symbol::new("Circle.draw")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Drawable", "draw")),
        )
        .expect(
            Symbol::new("Circle.area")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Shape", "area")),
        );
    }

    #[test]
    fn struct_implements_multiple_inherited_protocols() {
        // When conforming to Widget (which inherits from Drawable and Clickable),
        // must also explicitly conform to Drawable and Clickable
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
            }
            protocol Clickable {
                func onClick()
            }
            protocol Widget: Drawable, Clickable {
                func update()
            }
            struct Button: Drawable, Clickable, Widget {
                func draw() { }
                func onClick() { }
                func update() { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Button.draw")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Drawable", "draw")),
        )
        .expect(
            Symbol::new("Button.onClick")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Clickable", "onClick")),
        )
        .expect(
            Symbol::new("Button.update")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Widget", "update")),
        );
    }
}

mod self_type {
    use super::*;

    #[test]
    fn method_with_self_parameter() {
        Test::new(
            r#"module Test
            protocol Comparable {
                func compare(other: Self) -> lang.i1
            }
            struct Number: Comparable {
                func compare(other: Number) -> lang.i1 { true }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Number.compare")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Comparable", "compare")),
        );
    }

    #[test]
    fn method_with_self_return_type() {
        Test::new(
            r#"module Test
            protocol Cloneable {
                func clone() -> Self
            }
            struct Point: Cloneable {
                func clone() -> Point { Point() }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point.clone")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Cloneable", "clone")),
        );
    }

    #[test]
    fn method_with_self_in_array() {
        Test::new(
            r#"module Test
            protocol Collection {
                func getAll() -> [Self]
            }
            struct Item: Collection {
                func getAll() -> [Item] { [] }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Item.getAll")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Collection", "getAll")),
        );
    }
}

mod associated_types {
    use super::*;

    #[test]
    fn method_with_associated_type_parameter() {
        Test::new(
            r#"module Test
            protocol Container {
                type Item;
                func add(item: Item)
            }
            struct Box: Container {
                type Item = lang.i64;
                func add(item: lang.i64) { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Box.add")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Container", "add")),
        );
    }

    #[test]
    fn method_with_associated_type_return() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
                func next() -> Item
            }
            struct Counter: Iterator {
                type Item = lang.i64;
                func next() -> lang.i64 { 0 }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Counter.next")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Iterator", "next")),
        );
    }

    #[test]
    fn method_with_associated_type_in_array() {
        Test::new(
            r#"module Test
            protocol Collection {
                type Element;
                func getAll() -> [Element]
            }
            struct IntArray: Collection {
                type Element = lang.i64;
                func getAll() -> [lang.i64] { [] }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("IntArray.getAll")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Collection", "getAll")),
        );
    }
}

mod multiple_conformances {
    use super::*;

    #[test]
    fn struct_implements_methods_from_multiple_protocols() {
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
            Symbol::new("Button.draw")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Drawable", "draw")),
        )
        .expect(
            Symbol::new("Button.onClick")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Clickable", "onClick")),
        );
    }
}

mod errors {
    use super::*;

    #[test]
    fn ambiguous_method_satisfies_multiple_protocols() {
        Test::new(
            r#"module Test
            protocol Drawable {
                func render()
            }
            protocol Paintable {
                func render()
            }
            struct Canvas: Drawable, Paintable {
                func render() { }
            }
        "#,
        )
        .expect(HasError("ambiguous"))
        .expect(HasError("render"));
    }

    #[test]
    fn receiver_kind_mismatch_static_vs_instance() {
        Test::new(
            r#"module Test
            protocol Factory {
                static func create() -> Self
            }
            struct Item: Factory {
                func create() -> Item { }
            }
        "#,
        )
        .expect(HasError("receiver"))
        .expect(HasError("create"));
    }

    #[test]
    fn receiver_kind_mismatch_instance_vs_static() {
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
            }
            struct Circle: Drawable {
                static func draw() { }
            }
        "#,
        )
        .expect(HasError("receiver"))
        .expect(HasError("draw"));
    }
}

mod receiver_kinds {
    use super::*;

    #[test]
    fn static_method_implements_static_protocol_method() {
        Test::new(
            r#"module Test
            protocol Factory {
                static func create() -> lang.i64
            }
            struct Item: Factory {
                static func create() -> lang.i64 { 0 }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Item.create")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Factory", "create")),
        );
    }

    #[test]
    fn mutating_method_implements_mutating_protocol_method() {
        Test::new(
            r#"module Test
            protocol Incrementable {
                mutating func increment()
            }
            struct Counter: Incrementable {
                mutating func increment() { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Counter.increment")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Incrementable", "increment")),
        );
    }

    #[test]
    fn consuming_method_implements_consuming_protocol_method() {
        Test::new(
            r#"module Test
            protocol Disposable {
                consuming func dispose()
            }
            struct Resource: Disposable {
                consuming func dispose() { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Resource.dispose")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Disposable", "dispose")),
        );
    }
}

mod edge_cases {
    use super::*;

    #[test]
    fn empty_protocol_no_methods_to_link() {
        Test::new(
            r#"module Test
            protocol Marker { }
            struct Point: Marker {
                func draw() { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point.draw")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocolNone),
        );
    }

    #[test]
    fn struct_with_no_conformances() {
        Test::new(
            r#"module Test
            struct Point {
                func draw() { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point.draw")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocolNone),
        );
    }

    #[test]
    fn method_overload_different_labels() {
        Test::new(
            r#"module Test
            protocol Printer {
                func print(value value: lang.i64)
                func print(text text: lang.str)
            }
            struct Console: Printer {
                func print(value value: lang.i64) { }
                func print(text text: lang.str) { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Console.print")
                .is(SymbolKind::Function)
                .has(Behavior::ImplementsProtocol("Printer", "print")),
        );
    }
}
