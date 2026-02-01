use kestrel_test_suite::*;

// =============================================================================
// ASSOCIATED TYPES IN PROTOCOLS
// =============================================================================

mod protocol_declaration {
    use super::*;

    #[test]
    fn protocol_with_abstract_associated_type() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Iterator")
                .is(SymbolKind::Protocol)
                .has(Behavior::ChildCount(1)),
        )
        .expect(Symbol::new("Iterator.Item").is(SymbolKind::AssociatedType));
    }

    #[test]
    fn protocol_with_multiple_associated_types() {
        Test::new(
            r#"module Test
            protocol Dictionary {
                type Key;
                type Value;
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Dictionary")
                .is(SymbolKind::Protocol)
                .has(Behavior::ChildCount(2)),
        )
        .expect(Symbol::new("Dictionary.Key").is(SymbolKind::AssociatedType))
        .expect(Symbol::new("Dictionary.Value").is(SymbolKind::AssociatedType));
    }

    #[test]
    fn protocol_with_associated_type_and_methods() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
                func next() -> Item
                func hasNext() -> lang.i1
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Iterator")
                .is(SymbolKind::Protocol)
                .has(Behavior::ChildCount(3)),
        )
        .expect(Symbol::new("Iterator.Item").is(SymbolKind::AssociatedType))
        .expect(Symbol::new("Iterator.next").is(SymbolKind::Function))
        .expect(Symbol::new("Iterator.hasNext").is(SymbolKind::Function));
    }

    #[test]
    fn generic_protocol_with_associated_type() {
        Test::new(
            r#"module Test
            protocol Converter[From] {
                type Output;
                func convert(input: From) -> Output
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Converter")
                .is(SymbolKind::Protocol)
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::ChildCount(2)),
        )
        .expect(Symbol::new("Converter.Output").is(SymbolKind::AssociatedType));
    }
}

mod associated_type_constraints {
    use super::*;

    #[test]
    fn associated_type_with_single_bound() {
        Test::new(
            r#"module Test
            protocol Equatable { }
            protocol Container {
                type Item: Equatable;
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Container.Item").is(SymbolKind::AssociatedType));
    }

    #[test]
    fn associated_type_with_multiple_bounds() {
        Test::new(
            r#"module Test
            protocol Equatable { }
            protocol Hashable { }
            protocol Container {
                type Item: Equatable, Hashable;
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Container.Item").is(SymbolKind::AssociatedType));
    }

    #[test]
    fn associated_type_bound_must_be_protocol() {
        Test::new(
            r#"module Test
            struct NotAProtocol { }
            protocol Container {
                type Item: NotAProtocol;
            }
        "#,
        )
        .expect(HasError("bound must be a protocol"));
    }
}

mod associated_type_defaults {
    use super::*;

    #[test]
    fn associated_type_with_default() {
        Test::new(
            r#"module Test
            protocol Parser {
                type Output = lang.str;
                func parse() -> Output
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Parser.Output").is(SymbolKind::AssociatedType));
    }

    #[test]
    fn associated_type_with_constraint_and_default() {
        Test::new(
            r#"module Test
            protocol Equatable { }
            struct MyInt: Equatable { }
            protocol Container {
                type Item: Equatable = MyInt;
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Container.Item").is(SymbolKind::AssociatedType));
    }

    #[test]
    fn associated_type_default_must_satisfy_constraint() {
        Test::new(
            r#"module Test
            protocol Equatable { }
            struct NotEquatable { }
            protocol Container {
                type Item: Equatable = NotEquatable;
            }
        "#,
        )
        .expect(HasError("does not satisfy bound"));
    }
}

// =============================================================================
// ASSOCIATED TYPE BINDINGS IN STRUCTS
// =============================================================================

mod struct_binding {
    use super::*;

    #[test]
    fn struct_provides_associated_type() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
                func next() -> Item
            }
            struct IntIterator: Iterator {
                type Item = lang.i64;
                func next() -> lang.i64 { 0 }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("IntIterator")
                .is(SymbolKind::Struct)
                .has(Behavior::ConformanceCount(1)),
        )
        .expect(Symbol::new("IntIterator.Item").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn struct_provides_multiple_associated_types() {
        Test::new(
            r#"module Test
            protocol Dictionary {
                type Key;
                type Value;
                func read(key: Key) -> Value
            }
            struct StringIntMap: Dictionary {
                type Key = lang.str;
                type Value = lang.i64;
                func read(key: lang.str) -> lang.i64 { 0 }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("StringIntMap.Key").is(SymbolKind::TypeAlias))
        .expect(Symbol::new("StringIntMap.Value").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn struct_missing_associated_type() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
                func next() -> Item
            }
            struct BadIterator: Iterator {
                func next() -> lang.i64 { 0 }
            }
        "#,
        )
        .expect(HasError("does not provide associated type 'Item'"));
    }

    #[test]
    fn struct_associated_type_must_satisfy_constraint() {
        Test::new(
            r#"module Test
            protocol Equatable { }
            struct NotEquatable { }
            protocol Container {
                type Item: Equatable;
            }
            struct BadContainer: Container {
                type Item = NotEquatable;
            }
        "#,
        )
        .expect(HasError("does not satisfy bound"));
    }

    #[test]
    fn struct_uses_default_associated_type() {
        Test::new(
            r#"module Test
            protocol Parser {
                type Output = lang.str;
                func parse() -> Output
            }
            struct SimpleParser: Parser {
                func parse() -> lang.str { "" }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("SimpleParser")
                .is(SymbolKind::Struct)
                .has(Behavior::ConformanceCount(1)),
        );
    }

    #[test]
    fn struct_overrides_default_associated_type() {
        Test::new(
            r#"module Test
            protocol Parser {
                type Output = lang.str;
                func parse() -> Output
            }
            struct IntParser: Parser {
                type Output = lang.i64;
                func parse() -> lang.i64 { 0 }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("IntParser.Output").is(SymbolKind::TypeAlias));
    }
}

mod qualified_binding {
    use super::*;

    #[test]
    fn qualified_associated_type_binding() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
            }
            protocol Container {
                type Item;
            }
            struct Foo: Iterator, Container {
                type Iterator.Item = lang.i64;
                type Container.Item = lang.str;
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Foo")
                .is(SymbolKind::Struct)
                .has(Behavior::ConformanceCount(2)),
        );
    }

    #[test]
    fn ambiguous_associated_type_without_qualification() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
            }
            protocol Container {
                type Item;
            }
            struct Foo: Iterator, Container {
                type Item = lang.i64;
            }
        "#,
        )
        .expect(HasError("ambiguous associated type"));
    }

    #[test]
    fn qualified_binding_for_generic_protocol() {
        Test::new(
            r#"module Test
            protocol Add[Right] {
                type Output;
                func add(right: Right) -> Output
            }
            struct Int: Add[lang.i64] {
                type Add[lang.i64].Output = lang.i64;
                func add(right: lang.i64) -> lang.i64 { 0 }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn multiple_generic_protocol_conformances() {
        Test::new(
            r#"module Test
            protocol Add[Right] {
                type Output;
            }
            struct Int: Add[lang.i64], Add[lang.f64] {
                type Add[lang.i64].Output = lang.i64;
                type Add[lang.f64].Output = lang.f64;
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn qualified_binding_wrong_protocol() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
            }
            protocol Other { }
            struct Foo: Iterator, Other {
                type Other.Item = lang.i64;
            }
        "#,
        )
        .expect(HasError("does not have associated type 'Item'"));
    }

    #[test]
    fn qualified_binding_not_conforming() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
            }
            struct Foo {
                type Iterator.Item = lang.i64;
            }
        "#,
        )
        .expect(HasError("does not conform to 'Iterator'"));
    }
}

mod generic_struct_binding {
    use super::*;

    #[test]
    fn generic_struct_binds_type_parameter() {
        Test::new(
            r#"module Test
            protocol Container {
                type Item;
            }
            struct Box[T]: Container {
                type Item = T;
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Box")
                .is(SymbolKind::Struct)
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::ConformanceCount(1)),
        );
    }

    #[test]
    fn generic_struct_binds_transformed_type() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
            }
            struct ArrayIterator[T]: Iterator {
                type Item = T;
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn self_referential_associated_type() {
        Test::new(
            r#"module Test
            protocol Add[Right] {
                type Output;
                func add(right: Right) -> Output
            }
            struct Int: Add[lang.i64] {
                type Add[lang.i64].Output = lang.i64;
                func add(right: lang.i64) -> lang.i64 { 0 }
            }
        "#,
        )
        .expect(Compiles);
    }
}

// =============================================================================
// WHERE CLAUSE CONSTRAINTS ON ASSOCIATED TYPES
// =============================================================================

mod where_clause_bounds {
    use super::*;

    #[test]
    fn where_clause_associated_type_bound() {
        Test::new(
            r#"module Test
            protocol Equatable { }
            protocol Iterator {
                type Item;
            }
            func findEqual[T](iter: T) where T: Iterator, T.Item: Equatable { }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn where_clause_associated_type_multiple_bounds() {
        Test::new(
            r#"module Test
            protocol Equatable { }
            protocol Hashable { }
            protocol Iterator {
                type Item;
            }
            func process[T](iter: T) where T: Iterator, T.Item: Equatable, T.Item: Hashable { }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_inheritance_with_associated_type_constraint() {
        Test::new(
            r#"module Test
            protocol Comparable { }
            protocol Iterator {
                type Item;
            }
            protocol SortedIterator: Iterator where Iterator.Item: Comparable { }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn where_clause_associated_type_not_found() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
            }
            func process[T](iter: T) where T: Iterator, T.Unknown: Equatable { }
        "#,
        )
        .expect(HasError("no associated type 'Unknown'"));
    }
}

mod where_clause_equality {
    use super::*;

    #[test]
    fn where_clause_associated_type_equals_concrete() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
            }
            func intOnly[T](iter: T) where T: Iterator, T.Item = lang.i64 { }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn where_clause_associated_type_equals_type_parameter() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
                func next() -> Item
            }
            func collect[T, U](iter: T) -> U where T: Iterator, T.Item = U {
                iter.next()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn where_clause_two_associated_types_equal() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
            }
            func zip[A, B](a: A, b: B) where A: Iterator, B: Iterator, A.Item = B.Item { }
        "#,
        )
        .expect(Compiles);
    }
}

// =============================================================================
// ASSOCIATED TYPE RESOLUTION IN EXPRESSIONS
// =============================================================================

mod type_resolution {
    use super::*;

    #[test]
    fn method_returns_associated_type() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
                func next() -> Item
            }
            struct IntIterator: Iterator {
                type Item = lang.i64;
                func next() -> lang.i64 { 0 }
            }
            func test() {
                let iter = IntIterator();
                let x: lang.i64 = iter.next();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn method_takes_associated_type_parameter() {
        Test::new(
            r#"module Test
            protocol Container {
                type Item;
                func add(item: Item)
            }
            struct IntContainer: Container {
                type Item = lang.i64;
                func add(item: lang.i64) { }
            }
            func test() {
                let c = IntContainer();
                c.add(42);
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn generic_function_with_associated_type_constraint() {
        Test::new(
            r#"module Test
            protocol Equatable {
                func eq(other: Self) -> lang.i1
            }
            protocol Iterator {
                type Item;
                func next() -> Item
            }
            struct MyInt: Equatable {
                func eq(other: MyInt) -> lang.i1 { true }
            }
            struct IntIterator: Iterator {
                type Item = MyInt;
                func next() -> MyInt { MyInt() }
            }
            func contains[T](iter: T, value: T.Item) -> lang.i1 where T: Iterator, T.Item: Equatable {
                true
            }
        "#,
        )
        .expect(Compiles);
    }
}

// =============================================================================
// ERROR CASES
// =============================================================================

mod errors {
    use super::*;

    #[test]
    fn associated_type_without_equals_in_struct() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
            }
            struct Foo: Iterator {
                type Item;
            }
        "#,
        )
        .expect(HasError("associated type binding requires a type"));
    }

    #[test]
    fn type_alias_with_bounds_at_module_level() {
        Test::new(
            r#"module Test
            protocol Equatable { }
            type Foo: Equatable = lang.i64;
        "#,
        )
        .expect(HasError("type alias cannot have bounds"));
    }

    #[test]
    fn type_alias_without_equals_at_module_level() {
        Test::new(
            r#"module Test
            type Foo;
        "#,
        )
        .expect(HasError("type alias requires a type"));
    }

    #[test]
    fn associated_type_in_non_conforming_struct() {
        Test::new(
            r#"module Test
            struct Foo {
                type Item = lang.i64;
            }
        "#,
        )
        // This should be allowed - it's just a regular type alias in a struct
        .expect(Compiles)
        .expect(Symbol::new("Foo.Item").is(SymbolKind::TypeAlias));
    }
}

// =============================================================================
// PROTOCOL INHERITANCE
// =============================================================================

mod nested_associated_types {
    use super::*;

    #[test]
    fn nested_associated_type_access() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
            }
            protocol Container {
                type Iter: Iterator;
            }
            func getItem[C](c: C, item: C.Iter.Item) -> C.Iter.Item where C: Container { item }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn where_clause_on_nested_associated_type() {
        Test::new(
            r#"module Test
            protocol Equatable { }
            protocol Iterator {
                type Item;
            }
            protocol Container {
                type Iter: Iterator;
            }
            func findIn[C](c: C) where C: Container, C.Iter.Item: Equatable { }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn nested_associated_type_equality() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
            }
            protocol Container {
                type Iter: Iterator;
            }
            func intContainer[C](c: C) where C: Container, C.Iter.Item = lang.i64 { }
        "#,
        )
        .expect(Compiles);
    }
}

mod generic_defaults {
    use super::*;

    #[test]
    fn associated_type_with_generic_default() {
        Test::new(
            r#"module Test
            struct Array[T] { }
            protocol Collection[T] {
                type Storage = Array[T];
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Collection.Storage").is(SymbolKind::AssociatedType));
    }

    #[test]
    fn associated_type_default_uses_protocol_type_param() {
        Test::new(
            r#"module Test
            struct Pair[A, B] { }
            protocol Mapping[K, V] {
                type Entry = Pair[K, V];
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Mapping.Entry").is(SymbolKind::AssociatedType));
    }

    #[test]
    fn struct_overrides_generic_default() {
        Test::new(
            r#"module Test
            struct Array[T] { }
            struct LinkedList[T] { }
            protocol Collection[T] {
                type Storage = Array[T];
            }
            struct MyCollection[T]: Collection[T] {
                type Storage = LinkedList[T];
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("MyCollection.Storage").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn struct_uses_generic_default() {
        Test::new(
            r#"module Test
            struct Array[T] { }
            protocol Collection[T] {
                type Storage = Array[T];
            }
            struct SimpleCollection[T]: Collection[T] { }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn associated_type_default_invalid_type_arg() {
        Test::new(
            r#"module Test
            struct Array[T] { }
            protocol Collection[T] {
                type Storage = Array[Unknown];
            }
        "#,
        )
        .expect(HasError("cannot find type"));
    }
}

mod protocol_inheritance {
    use super::*;

    #[test]
    fn child_protocol_inherits_associated_type() {
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
            }
            protocol BidirectionalIterator: Iterator {
                func prev() -> Item
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("BidirectionalIterator")
                .is(SymbolKind::Protocol)
                .has(Behavior::ConformanceCount(1)),
        );
    }

    #[test]
    fn struct_conforming_to_child_provides_associated_type() {
        // When conforming to BidirectionalIterator (which inherits from Iterator),
        // must also explicitly conform to Iterator
        Test::new(
            r#"module Test
            protocol Iterator {
                type Item;
                func next() -> Item
            }
            protocol BidirectionalIterator: Iterator {
                func prev() -> Item
            }
            struct IntBiIterator: Iterator, BidirectionalIterator {
                type Item = lang.i64;
                func next() -> lang.i64 { 0 }
                func prev() -> lang.i64 { 0 }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_refines_associated_type_constraint() {
        Test::new(
            r#"module Test
            protocol Comparable { }
            protocol Iterator {
                type Item;
            }
            protocol SortedIterator: Iterator where Iterator.Item: Comparable {
                func min() -> Item
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_conforming_to_refined_protocol_must_satisfy_constraint() {
        Test::new(
            r#"module Test
            protocol Comparable { }
            protocol Iterator {
                type Item;
            }
            protocol SortedIterator: Iterator where Iterator.Item: Comparable { }
            struct NotComparable { }
            struct BadIterator: SortedIterator {
                type Item = NotComparable;
            }
        "#,
        )
        .expect(HasError("does not satisfy bound"));
    }
}

// =============================================================================
// MULTIPLE PROTOCOLS WITH SAME ASSOCIATED TYPE NAME
// =============================================================================

mod multiple_protocols_same_associated_type {
    use super::*;

    #[test]
    fn qualified_bindings_for_different_protocols() {
        // When a type conforms to multiple protocols that each have an associated type
        // with the same name, qualified bindings should correctly distinguish them.
        Test::new(
            r#"module Test
            protocol Addable[Rhs = Self] {
                type Output;
                func add(other: Rhs) -> Output
            }

            protocol RangeConstructible[Rhs = Self] {
                type Output;
                func exclusiveRange(to end: Rhs) -> Output
            }

            struct Range[T] {
                init() { }
            }

            struct MyInt: Addable, RangeConstructible {
                type Addable.Output = MyInt;
                type RangeConstructible.Output = Range[MyInt];

                init() { }

                func add(other: MyInt) -> MyInt { self }
                func exclusiveRange(to end: MyInt) -> Range[MyInt] { Range[MyInt]() }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn mismatched_return_type_with_qualified_bindings() {
        // Verify that using the wrong type for a method is caught even with qualified bindings
        Test::new(
            r#"module Test
            protocol Addable[Rhs = Self] {
                type Output;
                func add(other: Rhs) -> Output
            }

            protocol RangeConstructible[Rhs = Self] {
                type Output;
                func exclusiveRange(to end: Rhs) -> Output
            }

            struct Range[T] {
                init() { }
            }

            struct MyInt: Addable, RangeConstructible {
                type Addable.Output = MyInt;
                type RangeConstructible.Output = Range[MyInt];

                init() { }

                // Wrong: returns Range[MyInt] but Addable.Output = MyInt
                func add(other: MyInt) -> Range[MyInt] { Range[MyInt]() }
                func exclusiveRange(to end: MyInt) -> Range[MyInt] { Range[MyInt]() }
            }
        "#,
        )
        .expect(HasError("wrong return type"));
    }
}
