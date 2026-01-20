use kestrel_test_suite::*;

mod basic {
    use super::*;

    #[test]
    fn simple_enum() {
        Test::new(
            r#"module Test
            enum Color {
                case Red
                case Green
                case Blue
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Color").is(SymbolKind::Enum));
    }

    #[test]
    fn empty_enum() {
        Test::new(
            r#"module Test
            enum Empty { }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Empty").is(SymbolKind::Enum));
    }

    #[test]
    fn enum_with_single_case() {
        Test::new(
            r#"module Test
            enum Single {
                case Only
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Single").is(SymbolKind::Enum));
    }

    #[test]
    fn enum_visibility_modifiers() {
        Test::new(
            r#"module Test
            public enum Public {
                case Value
            }
            private enum Private {
                case Value
            }
            internal enum Internal {
                case Value
            }
            fileprivate enum Fileprivate {
                case Value
            }
            enum Default {
                case Value
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Public")
                .is(SymbolKind::Enum)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(
            Symbol::new("Private")
                .is(SymbolKind::Enum)
                .has(Behavior::Visibility(Visibility::Private)),
        )
        .expect(
            Symbol::new("Internal")
                .is(SymbolKind::Enum)
                .has(Behavior::Visibility(Visibility::Internal)),
        )
        .expect(
            Symbol::new("Fileprivate")
                .is(SymbolKind::Enum)
                .has(Behavior::Visibility(Visibility::Fileprivate)),
        )
        .expect(
            Symbol::new("Default")
                .is(SymbolKind::Enum)
                .has(Behavior::Visibility(Visibility::Internal)),
        );
    }

    #[test]
    fn multiple_enums() {
        Test::new(
            r#"module Test
            enum First {
                case A
            }
            enum Second {
                case B
            }
            enum Third {
                case C
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("First").is(SymbolKind::Enum))
        .expect(Symbol::new("Second").is(SymbolKind::Enum))
        .expect(Symbol::new("Third").is(SymbolKind::Enum));
    }
}

mod nested {
    use super::*;

    #[test]
    fn nested_enum() {
        Test::new(
            r#"module Test
            struct Container {
                enum Status {
                    case Active
                    case Inactive
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Container").is(SymbolKind::Struct))
        .expect(Symbol::new("Container.Status").is(SymbolKind::Enum));
    }

    #[test]
    fn deeply_nested_enums() {
        Test::new(
            r#"module Test
            struct Level1 {
                enum Level2 {
                    case Value
                    enum Level3 {
                        case DeepValue
                    }
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Level1").is(SymbolKind::Struct))
        .expect(Symbol::new("Level1.Level2").is(SymbolKind::Enum))
        .expect(Symbol::new("Level1.Level2.Level3").is(SymbolKind::Enum));
    }
}

mod associated_values {
    use super::*;

    #[test]
    fn case_with_single_labeled_parameter() {
        Test::new(
            r#"module Test
            enum Shape {
                case Circle(radius: lang.f64)
                case Point
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Shape").is(SymbolKind::Enum));
    }

    #[test]
    fn case_with_multiple_parameters() {
        Test::new(
            r#"module Test
            enum Shape {
                case Rectangle(width: lang.f64, height: lang.f64)
                case Circle(radius: lang.f64)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Shape").is(SymbolKind::Enum));
    }

    #[test]
    fn cases_with_different_types() {
        Test::new(
            r#"module Test
            enum Data {
                case Number(value: lang.i64)
                case Text(value: lang.str)
                case Flag(value: lang.i1)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Data").is(SymbolKind::Enum));
    }

    #[test]
    fn mixed_cases_with_and_without_values() {
        Test::new(
            r#"module Test
            enum Status {
                case Pending
                case InProgress(percentage: lang.i64)
                case Completed
                case Failed(reason: lang.str)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Status").is(SymbolKind::Enum));
    }
}

mod generic_enums {
    use super::*;

    #[test]
    fn single_type_parameter() {
        Test::new(
            r#"module Test
            enum Option[T] {
                case Some(value: T)
                case None
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Option")
                .is(SymbolKind::Enum)
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::IsGeneric(true)),
        );
    }

    #[test]
    fn multiple_type_parameters() {
        Test::new(
            r#"module Test
            enum Result[T, E] {
                case Ok(value: T)
                case Error(error: E)
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Result")
                .is(SymbolKind::Enum)
                .has(Behavior::TypeParamCount(2))
                .has(Behavior::IsGeneric(true)),
        );
    }

    #[test]
    fn generic_with_where_clause() {
        Test::new(
            r#"module Test
            protocol Equatable { }
            enum Set[T] where T: Equatable {
                case Empty
                case Elements(items: T)
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Set")
                .is(SymbolKind::Enum)
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::IsGeneric(true)),
        );
    }

    #[test]
    fn nested_generic_type_parameters() {
        Test::new(
            r#"module Test
            indirect enum Container[T] {
                case Single(value: T)
                case Nested(inner: Container[T])
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Container")
                .is(SymbolKind::Enum)
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::IsGeneric(true)),
        );
    }

    #[test]
    fn generic_with_type_parameter_defaults() {
        Test::new(
            r#"module Test
            enum Result[T, E = lang.str] {
                case Ok(value: T)
                case Error(error: E)
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Result")
                .is(SymbolKind::Enum)
                .has(Behavior::TypeParamCount(2))
                .has(Behavior::IsGeneric(true)),
        );
    }
}

mod recursive_enums {
    use super::*;

    #[test]
    fn indirect_enum_with_recursion() {
        Test::new(
            r#"module Test
            indirect enum Tree[T] {
                case Leaf(value: T)
                case Node(left: Tree[T], right: Tree[T])
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Tree")
                .is(SymbolKind::Enum)
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::IsGeneric(true)),
        );
    }

    #[test]
    fn indirect_linked_list() {
        Test::new(
            r#"module Test
            indirect enum List[T] {
                case Cons(head: T, tail: List[T])
                case Nil
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("List")
                .is(SymbolKind::Enum)
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::IsGeneric(true)),
        );
    }

    #[test]
    fn recursive_without_indirect_fails() {
        Test::new(
            r#"module Test
            enum Tree {
                case Leaf(value: lang.i64)
                case Node(left: Tree, right: Tree)
            }
        "#,
        )
        .expect(HasError("recursive enum requires `indirect`"));
    }

    #[test]
    fn recursive_single_self_reference_fails() {
        Test::new(
            r#"module Test
            enum Recursive {
                case Base
                case Next(value: Recursive)
            }
        "#,
        )
        .expect(HasError("recursive enum requires `indirect`"));
    }

    #[test]
    fn non_recursive_enum_compiles_without_indirect() {
        Test::new(
            r#"module Test
            enum Simple {
                case A
                case B(value: lang.i64)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Simple").is(SymbolKind::Enum));
    }

    #[test]
    fn recursive_through_struct_requires_indirect() {
        Test::new(
            r#"module Test
            enum Node {
                case Leaf
                case Branch(data: Box)
            }
            struct Box {
                var node: Node
            }
        "#,
        )
        .expect(HasError("recursive enum requires `indirect`"));
    }
}

mod future_features {
    use super::*;

    #[test]
    fn enum_methods() {
        Test::new(
            r#"module Test
            indirect enum LinkedList[T] {
                case Node(value: T, next: LinkedList[T])
                case Empty

                func length() -> lang.i64 { return 0; }
                static func createEmpty() -> LinkedList[T] { return .Empty; }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn enum_protocol_conformance() {
        Test::new(
            r#"module Test
            protocol Named {
                func name() -> lang.str
            }
            enum State: Named {
                case Active
                case Inactive
                func name() -> lang.str { return "State"; }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod instantiation {
    use super::*;

    #[test]
    fn full_path_simple_case() {
        Test::new(
            r#"module Test
            enum Color {
                case Red
                case Green
                case Blue
            }

            func getColor() -> Color {
                Color.Red
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Color").is(SymbolKind::Enum))
        .expect(Symbol::new("getColor").is(SymbolKind::Function));
    }

    #[test]
    fn full_path_with_associated_values() {
        Test::new(
            r#"module Test
            enum Shape {
                case Circle(radius: lang.f64)
                case Rectangle(width: lang.f64, height: lang.f64)
            }

            func getShape() -> Shape {
                Shape.Circle(radius: 5.0)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Shape").is(SymbolKind::Enum));
    }

    #[test]
    fn full_path_generic_enum() {
        Test::new(
            r#"module Test
            enum Option[T] {
                case Some(value: T)
                case None
            }

            func getSome() -> Option[lang.i64] {
                Option.Some(value: 42)
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Option")
                .is(SymbolKind::Enum)
                .has(Behavior::IsGeneric(true)),
        );
    }

    #[test]
    fn shorthand_with_type_annotation() {
        Test::new(
            r#"module Test
            enum Color {
                case Red
                case Green
                case Blue
            }

            func test() {
                let color: Color = .Red;
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Color").is(SymbolKind::Enum));
    }

    #[test]
    fn shorthand_with_associated_values() {
        Test::new(
            r#"module Test
            enum Shape {
                case Circle(radius: lang.f64)
                case Rectangle(width: lang.f64, height: lang.f64)
            }

            func test() {
                let shape: Shape = .Circle(radius: 5.0);
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Shape").is(SymbolKind::Enum));
    }

    #[test]
    fn shorthand_in_function_argument() {
        Test::new(
            r#"module Test
            enum Color {
                case Red
                case Green
                case Blue
            }

            func draw(color: Color) { }

            func test() {
                draw(.Red)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Color").is(SymbolKind::Enum));
    }

    #[test]
    fn shorthand_in_return_statement() {
        Test::new(
            r#"module Test
            enum Color {
                case Red
                case Green
                case Blue
            }

            func defaultColor() -> Color {
                return .Blue
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Color").is(SymbolKind::Enum));
    }

    #[test]
    fn shorthand_in_assignment() {
        Test::new(
            r#"module Test
            enum Status {
                case Pending
                case Active
                case Complete
            }

            func test() {
                var status: Status = .Pending;
                status = .Active;
                status = .Complete;
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Status").is(SymbolKind::Enum));
    }

    #[test]
    fn instantiation_with_multiple_associated_values() {
        Test::new(
            r#"module Test
            enum Event {
                case Click(x: lang.i64, y: lang.i64)
                case Scroll(delta: lang.f64)
            }

            func createEvent() -> Event {
                Event.Click(x: 100, y: 200)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Event").is(SymbolKind::Enum));
    }

    #[test]
    fn generic_enum_with_explicit_type_args() {
        Test::new(
            r#"module Test
            enum Option[T] {
                case Some(value: T)
                case None
            }

            func test() {
                let x = Option[lang.i64].None;
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Option").is(SymbolKind::Enum));
    }
}

mod error_unknown_case {
    use super::*;

    #[test]
    fn unknown_case_on_enum() {
        Test::new(
            r#"module Test
            enum Color {
                case Red
                case Green
                case Blue
            }

            func test() -> Color {
                Color.Purple
            }
        "#,
        )
        .expect(HasError("undefined name"));
    }

    #[test]
    fn unknown_case_shorthand() {
        Test::new(
            r#"module Test
            enum Color {
                case Red
                case Green
                case Blue
            }

            func test() {
                let color: Color = .Purple;
            }
        "#,
        )
        .expect(HasError("member not found"));
    }

    #[test]
    fn typo_in_case_name() {
        Test::new(
            r#"module Test
            enum Status {
                case Active
                case Inactive
            }

            func test() -> Status {
                Status.Actve
            }
        "#,
        )
        .expect(HasError("undefined name"));
    }
}

mod error_missing_wrong_label {
    use super::*;

    #[test]
    fn missing_label() {
        Test::new(
            r#"module Test
            enum Shape {
                case Circle(radius: lang.f64)
            }

            func test() -> Shape {
                Shape.Circle(5.0)
            }
        "#,
        )
        .expect(HasError("no matching overload"));
    }

    #[test]
    fn wrong_label() {
        Test::new(
            r#"module Test
            enum Shape {
                case Circle(radius: lang.f64)
            }

            func test() -> Shape {
                Shape.Circle(r: 5.0)
            }
        "#,
        )
        .expect(HasError("no matching overload"));
    }

    #[test]
    fn missing_label_multiple_params() {
        Test::new(
            r#"module Test
            enum Shape {
                case Rectangle(width: lang.f64, height: lang.f64)
            }

            func test() -> Shape {
                Shape.Rectangle(10.0, 20.0)
            }
        "#,
        )
        .expect(HasError("no matching overload"));
    }

    #[test]
    fn one_correct_one_wrong_label() {
        Test::new(
            r#"module Test
            enum Shape {
                case Rectangle(width: lang.f64, height: lang.f64)
            }

            func test() -> Shape {
                Shape.Rectangle(width: 10.0, h: 20.0)
            }
        "#,
        )
        .expect(HasError("no matching overload"));
    }

    #[test]
    fn shorthand_missing_label() {
        Test::new(
            r#"module Test
            enum Shape {
                case Circle(radius: lang.f64)
            }

            func draw(shape: Shape) { }

            func test() {
                draw(.Circle(5.0))
            }
        "#,
        )
        .expect(HasError("no matching overload"));
    }
}

mod error_cannot_infer_shorthand {
    use super::*;

    #[test]
    fn shorthand_without_type_context() {
        Test::new(
            r#"module Test
            enum Color {
                case Red
                case Green
                case Blue
            }

            func test() {
                let x = .Red;
            }
        "#,
        )
        .expect(HasError("cannot infer enum type for shorthand"));
    }

    #[test]
    fn shorthand_in_ambiguous_context() {
        Test::new(
            r#"module Test
            enum Color {
                case Red
            }

            enum TrafficLight {
                case Red
            }

            func test() {
                let x = .Red;
            }
        "#,
        )
        .expect(HasError("cannot infer enum type for shorthand"));
    }

    #[test]
    fn shorthand_with_associated_values_no_context() {
        Test::new(
            r#"module Test
            enum Shape {
                case Circle(radius: lang.f64)
            }

            func test() {
                let x = .Circle(radius: 5.0);
            }
        "#,
        )
        .expect(HasError("cannot infer enum type for shorthand"));
    }
}

mod error_type_mismatch {
    use super::*;

    #[test]
    fn wrong_type_for_associated_value() {
        Test::new(
            r#"module Test
            enum Shape {
                case Circle(radius: lang.f64)
            }

            func test() -> Shape {
                Shape.Circle(radius: "big")
            }
        "#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn wrong_type_multiple_params() {
        Test::new(
            r#"module Test
            enum Event {
                case Click(x: lang.i64, y: lang.i64)
            }

            func test() -> Event {
                Event.Click(x: 10, y: "twenty")
            }
        "#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn wrong_type_in_generic_enum() {
        Test::new(
            r#"module Test
            enum Option[T] {
                case Some(value: T)
                case None
            }

            func test() -> Option[lang.i64] {
                Option.Some(value: "hello")
            }
        "#,
        )
        .expect(HasError("type mismatch"));
    }
}

mod error_wrong_arity {
    use super::*;

    #[test]
    fn too_few_associated_values() {
        Test::new(
            r#"module Test
            enum Shape {
                case Rectangle(width: lang.f64, height: lang.f64)
            }

            func test() -> Shape {
                Shape.Rectangle(width: 5.0)
            }
        "#,
        )
        .expect(HasError("no matching overload"));
    }

    #[test]
    fn too_many_associated_values() {
        Test::new(
            r#"module Test
            enum Shape {
                case Circle(radius: lang.f64)
            }

            func test() -> Shape {
                Shape.Circle(radius: 5.0, extra: 10.0)
            }
        "#,
        )
        .expect(HasError("no matching overload"));
    }

    #[test]
    fn providing_values_to_valueless_case() {
        Test::new(
            r#"module Test
            enum Color {
                case Red
                case Green
            }

            func test() -> Color {
                Color.Red()
            }
        "#,
        )
        .expect(Compiles); // Color.Red() is valid - empty parens are allowed
    }

    #[test]
    fn missing_all_required_values() {
        Test::new(
            r#"module Test
            enum Point {
                case Location(x: lang.i64, y: lang.i64)
            }

            func test() -> Point {
                Point.Location()
            }
        "#,
        )
        .expect(HasError("no matching overload"));
    }
}

mod error_duplicate_case_name {
    use super::*;

    #[test]
    fn duplicate_case_in_same_enum() {
        Test::new(
            r#"module Test
            enum Color {
                case Red
                case Red
            }
        "#,
        )
        .expect(HasError("duplicate enum case"));
    }

    #[test]
    fn duplicate_case_different_associated_values() {
        Test::new(
            r#"module Test
            enum Shape {
                case Circle(radius: lang.f64)
                case Circle(diameter: lang.f64)
            }
        "#,
        )
        .expect(HasError("duplicate enum case"));
    }

    #[test]
    fn duplicate_case_one_with_values_one_without() {
        Test::new(
            r#"module Test
            enum Status {
                case Active
                case Active(reason: lang.str)
            }
        "#,
        )
        .expect(HasError("duplicate enum case"));
    }
}

mod error_duplicate_label {
    use super::*;

    #[test]
    fn duplicate_label_in_case() {
        Test::new(
            r#"module Test
            enum Bad {
                case Foo(x: lang.i64, x: lang.str)
            }
        "#,
        )
        .expect(HasError("duplicate label"));
    }

    #[test]
    fn duplicate_label_same_type() {
        Test::new(
            r#"module Test
            enum Point {
                case Location(x: lang.i64, x: lang.i64)
            }
        "#,
        )
        .expect(HasError("duplicate label"));
    }

    #[test]
    fn duplicate_label_three_params() {
        Test::new(
            r#"module Test
            enum Bad {
                case Triple(a: lang.i64, b: lang.str, a: lang.i1)
            }
        "#,
        )
        .expect(HasError("duplicate label"));
    }
}

mod edge_cases {
    use super::*;

    #[test]
    fn enum_with_many_cases() {
        Test::new(
            r#"module Test
            enum Alphabet {
                case A
                case B
                case C
                case D
                case E
                case F
                case G
                case H
                case I
                case J
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Alphabet").is(SymbolKind::Enum));
    }

    #[test]
    fn enum_case_with_enum_type_parameter() {
        Test::new(
            r#"module Test
            enum Inner {
                case Value
            }

            enum Outer {
                case Contains(inner: Inner)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Inner").is(SymbolKind::Enum))
        .expect(Symbol::new("Outer").is(SymbolKind::Enum));
    }

    #[test]
    fn generic_enum_with_protocol_constraint() {
        Test::new(
            r#"module Test
            protocol Hashable { }

            enum Set[T] where T: Hashable {
                case Empty
                case NonEmpty(value: T)
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Set")
                .is(SymbolKind::Enum)
                .has(Behavior::IsGeneric(true)),
        );
    }

    #[test]
    fn enum_with_generic_associated_value() {
        Test::new(
            r#"module Test
            enum Container[T] {
                case Single(value: T)
                case Multiple(values: T)
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Container")
                .is(SymbolKind::Enum)
                .has(Behavior::IsGeneric(true)),
        );
    }

    #[test]
    fn multiple_enums_same_case_names_different_scopes() {
        Test::new(
            r#"module Test
            enum A {
                case Value
            }

            enum B {
                case Value
            }

            func test() {
                let a: A = .Value;
                let b: B = .Value;
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("A").is(SymbolKind::Enum))
        .expect(Symbol::new("B").is(SymbolKind::Enum));
    }

    // NOTE: indirect_keyword_as_identifier_in_different_context test removed
    // because `indirect` is now a reserved keyword (not a contextual keyword)

    #[test]
    fn case_keyword_not_valid_as_identifier() {
        Test::new(
            r#"module Test
            func case() -> lang.i64 {
                42
            }
        "#,
        )
        .expect(Fails);
    }
}

mod regression {
    use super::*;

    /// Regression test for: Inline tuple in `.Some()` fails type inference with generic functions
    /// Issue: When passing `.Some((tuple))` to a generic function like `func[T](opt: Option[T])`,
    /// the compiler failed to infer that T should be the tuple type. This happened because:
    /// 1. The `infer_from_type` function didn't have a case for `TyKind::Enum` to match enum type arguments
    /// 2. Type parameters that couldn't be inferred weren't getting Infer substitutions, leaving TypeParameter types
    #[test]
    fn implicit_member_with_tuple_in_generic_function() {
        Test::new(
            r#"module Test
            public enum Option[T] {
                case Some(T)
                case None
            }

            // Generic function that takes Option[T]
            public func process[T](opt: Option[T]) -> lang.i64 {
                match opt {
                    .Some(_) => 1,
                    .None => 0
                }
            }

            // Test: inline tuple in .Some() with generic function
            public func test1() -> lang.i64 {
                process(.Some((5, 10)))
            }

            // Test: identity function with implicit member
            public func identity[T](x: Option[T]) -> Option[T] {
                x
            }

            public func test2() -> Option[(lang.i64, lang.i64)] {
                identity(.Some((5, 10)))
            }

            // Test: with more complex tuple types
            public func test3() -> lang.i64 {
                process(.Some((1, 2, 3)))
            }
        "#,
        )
        .expect(Compiles);
    }

    /// Regression test for: Enum cases inherit parent enum's visibility
    /// Issue: Enum cases were hardcoded to have Internal visibility, making public enum cases
    /// inaccessible when imported from another module. This caused name resolution errors like
    /// "'Equal' is not a type" or "undefined name 'Equal'" when trying to use enum cases.
    /// Root cause: EnumCaseBuilder hardcoded visibility to Internal instead of inheriting from parent.
    /// Fix: Enum cases now get their visibility from the parent enum's VisibilityBehavior.
    #[test]
    fn enum_cases_inherit_parent_visibility() {
        // Verify enum cases have same visibility as parent in single module
        Test::new(
            r#"module Test
            public enum PublicEnum {
                case PublicCase
            }

            private enum PrivateEnum {
                case PrivateCase
            }

            internal enum InternalEnum {
                case InternalCase
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("PublicEnum")
                .is(SymbolKind::Enum)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(
            Symbol::new("PublicEnum.PublicCase")
                .is(SymbolKind::EnumCase)
                .has(Behavior::Visibility(Visibility::Public)),
        )
        .expect(
            Symbol::new("PrivateEnum")
                .is(SymbolKind::Enum)
                .has(Behavior::Visibility(Visibility::Private)),
        )
        .expect(
            Symbol::new("PrivateEnum.PrivateCase")
                .is(SymbolKind::EnumCase)
                .has(Behavior::Visibility(Visibility::Private)),
        )
        .expect(
            Symbol::new("InternalEnum")
                .is(SymbolKind::Enum)
                .has(Behavior::Visibility(Visibility::Internal)),
        )
        .expect(
            Symbol::new("InternalEnum.InternalCase")
                .is(SymbolKind::EnumCase)
                .has(Behavior::Visibility(Visibility::Internal)),
        );
    }

    /// Regression test for: Public enum cases accessible across modules
    /// This is part of the same fix - ensures that public enum cases can be imported and used
    /// in other modules, which was the original symptom of the bug.
    #[test]
    fn public_enum_cases_accessible_across_modules() {
        Test::with_files(&[
            (
                "ordering.ks",
                r#"module Ordering
                public enum Ordering {
                    case Less
                    case Equal
                    case Greater
                }
            "#,
            ),
            (
                "consumer.ks",
                r#"module Consumer
                import Ordering.(Ordering)

                public func compare(a: lang.i64, b: lang.i64) -> Ordering {
                    if lang.i64_signed_lt(a, b) {
                        Ordering.Less
                    } else if lang.i64_signed_gt(a, b) {
                        Ordering.Greater
                    } else {
                        Ordering.Equal
                    }
                }
            "#,
            ),
        ])
        .expect(Compiles);
    }
}
