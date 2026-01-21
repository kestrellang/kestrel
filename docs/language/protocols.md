# Protocols

Protocols define a set of requirements (methods, properties, and associated types) that a type must fulfill to conform to the protocol. They enable polymorphism and generic programming in Kestrel.

## Declaration Syntax

### Basic Protocol

```kestrel
protocol Drawable {
    func draw()
}
```

### Protocol with Methods

```kestrel
protocol Comparable {
    func lessThan(other: Self) -> Bool
    func greaterThan(other: Self) -> Bool
    func equals(other: Self) -> Bool
}
```

### Public Protocol

```kestrel
public protocol Equatable {
    func equals(other: Self) -> Bool
}
```

### Empty (Marker) Protocol

```kestrel
protocol Marker { }
```

## Method Requirements

Protocols can require instance methods, static methods, and initializers.

### Instance Methods

Instance methods are declared without the `self` parameter (like all methods in Kestrel):

```kestrel
protocol Hashable {
    func hash() -> Int
}
```

### Receiver Kinds

Methods can specify different receiver kinds:

```kestrel
protocol Counter {
    // Borrowing (default) - read-only access to self
    func value() -> Int

    // Mutating - can modify self
    mutating func increment()

    // Consuming - takes ownership of self
    consuming func dispose()
}
```

### Static Methods

```kestrel
protocol Factory {
    static func create() -> Self
}
```

### Initializers

```kestrel
protocol ExpressibleByIntegerLiteral {
    init(intLiteral value: Int)
}
```

### Labeled Parameters

Method requirements can use labeled parameters:

```kestrel
protocol Greetable {
    func greet(with name: String)
}

// Implementation must match labels exactly
struct Person: Greetable {
    func greet(with name: String) { }
}
```

## Associated Types

Protocols can define associated types that conforming types must specify:

```kestrel
protocol Iterator {
    type Item
    func next() -> Item
}

struct Counter: Iterator {
    type Item = Int
    func next() -> Int { 0 }
}
```

### Associated Types with Defaults

```kestrel
protocol Parser {
    type Output = String
}
```

### Associated Types with Bounds

```kestrel
protocol Container {
    type Element: Equatable
    func contains(element: Element) -> Bool
}
```

### Using Self in Associated Types

```kestrel
protocol Collection {
    type Element
    func getAll() -> [Element]
}
```

## Generic Protocols

Protocols can have type parameters:

```kestrel
protocol Container[T] {
    func add(item: T)
    func read() -> T
}

struct Box[T]: Container[T] {
    var value: T

    func add(item: T) { self.value = item }
    func read() -> T { self.value }
}
```

### Multiple Type Parameters

```kestrel
protocol Mapping[K, V] {
    func get(key: K) -> V
    func set(key: K, value: V)
}
```

### Generic Protocols with Defaults

```kestrel
protocol Multipliable[Rhs = Self] {
    func multiply(other: Rhs) -> Self
}

struct Number: Multipliable {
    func multiply(other: Number) -> Number { ... }
}
```

## Protocol Inheritance

Protocols can inherit from other protocols:

```kestrel
protocol Drawable { }

protocol Shape: Drawable {
    func area() -> Int
}
```

### Multiple Inheritance

```kestrel
protocol Drawable { }
protocol Clickable { }

protocol Widget: Drawable, Clickable { }
```

### Inherited Methods

Types conforming to a child protocol must implement all methods from both the child and parent protocols:

```kestrel
protocol Drawable {
    func draw()
}

protocol Shape: Drawable {
    func area() -> Int
}

// Must implement both draw() and area()
// Must also explicitly conform to Drawable
struct Circle: Drawable, Shape {
    func draw() { }
    func area() -> Int { 42 }
}
```

## Conformance

### Struct Conformance

```kestrel
struct Point: Drawable {
    func draw() { }
}
```

### Multiple Conformances

```kestrel
protocol Drawable { }
protocol Equatable { }

struct Point: Drawable, Equatable {
    // Must implement all required methods
}
```

### Generic Conformance

```kestrel
struct Box[T]: Container[T] {
    // Implementation
}
```

### Conformance with Where Clause

```kestrel
struct Set[T]: Container[T] where T: Equatable {
    // Implementation
}
```

### Negative Conformance

Types can explicitly opt out of automatic protocol conformance:

```kestrel
struct File: not Copyable {
    // This type cannot be copied
}
```

This is primarily used with the `Copyable` protocol to indicate move-only types.

## Extensions

### Adding Conformance via Extension

```kestrel
struct Point { var x: Int; var y: Int }

protocol Hashable {
    func hash() -> Int
}

extend Point: Hashable {
    func hash() -> Int {
        x + y * 31
    }
}
```

### Multiple Conformances

```kestrel
extend Point: Hashable, Equatable {
    func hash() -> Int { x + y * 31 }
    func equals(other: Point) -> Bool { x == other.x && y == other.y }
}
```

### Generic Extensions

```kestrel
struct Box[T] { var value: T }

extend Box[T] {
    func getValue() -> T { self.value }
}
```

### Specialized Extensions

```kestrel
// Extension only for Box[Int]
extend Box[Int] {
    func doubled() -> Int { self.value * 2 }
}
```

### Constrained Extensions

```kestrel
extend Box[T] where T: Equatable {
    func contains(item: T) -> Bool {
        self.value.equals(item)
    }
}
```

## Protocol Extensions

Protocols themselves can be extended to provide default implementations:

```kestrel
protocol Drawable {
    func draw()
}

extend Drawable {
    func clear() { }
    func redraw() {
        self.clear()
        self.draw()
    }
}

struct Circle: Drawable {
    func draw() { }
    // Gets clear() and redraw() for free
}
```

### Constrained Protocol Extensions

```kestrel
protocol Filterable {
    func filter()
}

protocol Sortable {
    func sort()
}

extend Filterable where Self: Sortable {
    func filterAndSort() {
        self.filter()
        self.sort()
    }
}
```

### Protocol Extension Specificity

When multiple protocol extensions provide the same method, the most constrained one is used:

```kestrel
extend Filterable {
    func process() { /* generic version */ }
}

extend Filterable where Self: Sortable {
    func process() { /* specialized version - wins when type conforms to both */ }
}
```

## Built-in Protocols

Kestrel provides several built-in protocols that have special compiler support, marked with the `@builtin` attribute.

### Copyable

The `Copyable` protocol indicates that a type can be copied by value. Most types in Kestrel implicitly conform to `Copyable` unless they opt out:

```kestrel
@builtin(.Copyable)
protocol Copyable { }

// Explicitly opt out
struct File: not Copyable {
    // Move-only type
}
```

### Cloneable

For types that support explicit cloning:

```kestrel
protocol Cloneable {
    func clone() -> Self
}
```

### Equatable

For types that support equality comparison:

```kestrel
protocol Equatable {
    func equals(other: Self) -> Bool
}
```

### Comparable

For types that support ordering:

```kestrel
protocol Comparable {
    func lessThan(other: Self) -> Bool
}
```

### Literal Protocols

These protocols enable custom types to be created from literals:

```kestrel
@builtin(.ExpressibleByIntLiteral)
protocol ExpressibleByIntegerLiteral {
    init(intLiteral value: Int)
}

@builtin(.ExpressibleByFloatLiteral)
protocol ExpressibleByFloatLiteral {
    init(floatLiteral value: Float64)
}

@builtin(.ExpressibleByStringLiteral)
protocol ExpressibleByStringLiteral {
    init(stringLiteral value: String)
}

@builtin(.ExpressibleByBoolLiteral)
protocol ExpressibleByBoolLiteral {
    init(boolLiteral value: Bool)
}

@builtin(.ExpressibleByNilLiteral)
protocol ExpressibleByNilLiteral {
    init(nilLiteral value: ())
}
```

Example usage:

```kestrel
struct Percentage: ExpressibleByIntegerLiteral {
    var value: Int

    init(intLiteral value: Int) {
        self.value = value
    }
}

let p: Percentage = 50;  // Uses intLiteral initializer
```

### Operator Protocols

Operators in Kestrel are protocol-based. Types implement operators by conforming to operator protocols:

```kestrel
// Arithmetic operators
protocol AddOperatorProtocol {
    func add(rhs: Self) -> Self
}

protocol SubtractOperatorProtocol {
    func subtract(rhs: Self) -> Self
}

protocol MultiplyOperatorProtocol {
    func multiply(rhs: Self) -> Self
}

protocol DivideOperatorProtocol {
    func divide(rhs: Self) -> Self
}

protocol ModuloOperatorProtocol {
    func modulo(rhs: Self) -> Self
}

// Comparison operators
protocol EqualsOperatorProtocol {
    func equals(rhs: Self) -> Bool
}

protocol NotEqualsOperatorProtocol {
    func notEquals(rhs: Self) -> Bool
}

protocol LessThanOperatorProtocol {
    func lessThan(rhs: Self) -> Bool
}

protocol GreaterThanOperatorProtocol {
    func greaterThan(rhs: Self) -> Bool
}

protocol LessOrEqualOperatorProtocol {
    func lessThanOrEqual(rhs: Self) -> Bool
}

protocol GreaterOrEqualOperatorProtocol {
    func greaterThanOrEqual(rhs: Self) -> Bool
}

// Bitwise operators
protocol BitwiseAndOperatorProtocol {
    func bitwiseAnd(rhs: Self) -> Self
}

protocol BitwiseOrOperatorProtocol {
    func bitwiseOr(rhs: Self) -> Self
}

protocol BitwiseXorOperatorProtocol {
    func bitwiseXor(rhs: Self) -> Self
}

protocol BitwiseNotOperatorProtocol {
    func bitwiseNot() -> Self
}

protocol ShiftLeftOperatorProtocol {
    func shiftLeft(rhs: Self) -> Self
}

protocol ShiftRightOperatorProtocol {
    func shiftRight(rhs: Self) -> Self
}

// Unary operators
protocol NegateOperatorProtocol {
    func negate() -> Self
}

protocol LogicalNotOperatorProtocol {
    func logicalNot() -> Bool
}
```

Example implementation:

```kestrel
struct Number: AddOperatorProtocol, SubtractOperatorProtocol {
    var value: Int

    func add(rhs: Number) -> Number {
        Number(value: self.value + rhs.value)
    }

    func subtract(rhs: Number) -> Number {
        Number(value: self.value - rhs.value)
    }
}

let a = Number(value: 5)
let b = Number(value: 3)
let sum = a + b      // Calls a.add(b)
let diff = a - b     // Calls a.subtract(b)
```

## The Self Type

The `Self` type refers to the conforming type:

```kestrel
protocol Equatable {
    func equals(other: Self) -> Bool
}

protocol Cloneable {
    func clone() -> Self
}
```

When a concrete type conforms, `Self` is replaced with that type:

```kestrel
struct Point: Equatable {
    var x: Int
    var y: Int

    // Self = Point
    func equals(other: Point) -> Bool {
        x == other.x && y == other.y
    }
}
```

## Validation Rules

### Conformance Validation

A type must implement all required methods to conform to a protocol:

```kestrel
protocol Drawable {
    func draw()
    func clear()
}

struct Circle: Drawable {
    func draw() { }
    // Error: does not implement method 'clear'
}
```

### Method Signature Matching

Implementations must exactly match the protocol requirement:

```kestrel
protocol Hashable {
    func hash() -> Int
}

struct Point: Hashable {
    func hash() -> String { "" }
    // Error: method 'hash' has wrong return type
}
```

### Label Matching

Parameter labels must match:

```kestrel
protocol Greetable {
    func greet(with name: String)
}

struct Person: Greetable {
    func greet(using name: String) { }
    // Error: does not implement method 'greet'
}
```

### Receiver Kind Matching

The receiver kind must match:

```kestrel
protocol Factory {
    static func create() -> Self
}

struct Item: Factory {
    func create() -> Item { }  // Missing 'static'
    // Error: receiver kind mismatch
}
```

### Inherited Protocol Requirements

When conforming to a child protocol, must also explicitly conform to parent protocols:

```kestrel
protocol Drawable {
    func draw()
}

protocol Shape: Drawable {
    func area() -> Int
}

struct Circle: Shape {
    func draw() { }
    func area() -> Int { 42 }
}
// Error: conforms to 'Shape' but not its parent protocol 'Drawable'

// Correct:
struct Circle: Drawable, Shape {
    func draw() { }
    func area() -> Int { 42 }
}
```

### Associated Type Conflicts

Diamond inheritance with conflicting associated types is an error:

```kestrel
protocol A {
    type Element
}

protocol B {
    type Element
}

protocol C: A, B { }
// Error: conflicting associated type 'Element'
```

However, child protocols can redeclare parent associated types:

```kestrel
protocol Parent {
    type Element
}

protocol Child: Parent {
    type Element  // OK - refining parent's associated type
}
```

## Type-Directed Conformance

When a type has multiple initializers with the same label but different parameter types, the compiler selects based on the argument type:

```kestrel
protocol Convertible[T] {
    init(from other: T)
}

struct Target: Convertible[Small], Convertible[Large] {
    init(from other: Small) { ... }
    init(from other: Large) { ... }
}

let s = Small()
let t = Target(from: s)  // Calls init(from: Small)
```

## Errors

Common protocol-related errors:

| Error | Message |
|-------|---------|
| Missing method | "does not implement method 'methodName'" |
| Wrong return type | "method 'methodName' has wrong return type" |
| Wrong parameter count | "does not implement method 'methodName'" |
| Wrong label | "does not implement method 'methodName'" |
| Receiver kind mismatch | "receiver kind mismatch" |
| Missing parent conformance | "conforms to 'Child' but not its parent protocol 'Parent'" |
| Conflicting associated type | "conflicting associated type 'TypeName'" |
| Duplicate protocol method | "ambiguous method 'methodName'" |
| Missing associated type | "does not provide associated type 'TypeName'" |

## Grammar

```ebnf
protocol_declaration =
    visibility? "protocol" identifier type_parameter_list? protocol_inheritance? where_clause? "{" protocol_body "}"

protocol_inheritance =
    ":" protocol_conformance_list

protocol_conformance_list =
    protocol_conformance ("," protocol_conformance)*

protocol_conformance =
    type_expr

protocol_body =
    protocol_member*

protocol_member =
    | method_requirement
    | initializer_requirement
    | associated_type
    | static_method_requirement

method_requirement =
    receiver_kind? "func" identifier parameter_list return_type?

initializer_requirement =
    generic_clause? "init" parameter_list

associated_type =
    "type" identifier (":" type_bound_list)? ("=" type_expr)?

receiver_kind =
    | "mutating"
    | "consuming"
    | "static"

conformance_declaration =
    type_name ":" protocol_conformance_list

extension_conformance =
    "extend" type_expr (":" protocol_conformance_list)? where_clause? "{" extension_body "}"

protocol_extension =
    "extend" protocol_name where_clause? "{" extension_body "}"

negative_conformance =
    "not" identifier

builtin_attribute =
    "@builtin" "(" "." builtin_feature ")"

builtin_feature =
    | "Copyable"
    | "ExpressibleByIntLiteral"
    | "ExpressibleByFloatLiteral"
    | "ExpressibleByStringLiteral"
    | "ExpressibleByBoolLiteral"
    | "ExpressibleByNilLiteral"
```

## Examples

### Complete Example: Collection Protocol

```kestrel
protocol Collection {
    type Element

    func count() -> Int
    func isEmpty() -> Bool
    func contains(element: Element) -> Bool where Element: Equatable
}

extend Collection {
    // Default implementation
    func isEmpty() -> Bool {
        self.count() == 0
    }
}

struct IntArray: Collection {
    type Element = Int
    var items: [Int]

    func count() -> Int { items.length }
    func contains(element: Int) -> Bool { ... }
}
```

### Complete Example: Operator Overloading

```kestrel
struct Vector: AddOperatorProtocol, SubtractOperatorProtocol, EqualsOperatorProtocol {
    var x: Int
    var y: Int

    func add(rhs: Vector) -> Vector {
        Vector(x: self.x + rhs.x, y: self.y + rhs.y)
    }

    func subtract(rhs: Vector) -> Vector {
        Vector(x: self.x - rhs.x, y: self.y - rhs.y)
    }

    func equals(rhs: Vector) -> Bool {
        self.x == rhs.x && self.y == rhs.y
    }
}

let v1 = Vector(x: 1, y: 2)
let v2 = Vector(x: 3, y: 4)
let sum = v1 + v2           // Vector(x: 4, y: 6)
let diff = v1 - v2          // Vector(x: -2, y: -2)
let same = v1 == v2         // false
```

### Complete Example: Generic Protocol with Constraints

```kestrel
protocol Iterator {
    type Item
    mutating func next() -> Item?
}

protocol Iterable {
    type Iter: Iterator
    func iter() -> Iter
}

struct Range: Iterable {
    type Iter = RangeIterator
    var start: Int
    var end: Int

    func iter() -> RangeIterator {
        RangeIterator(current: start, end: end)
    }
}

struct RangeIterator: Iterator {
    type Item = Int
    var current: Int
    var end: Int

    mutating func next() -> Int? {
        if current < end {
            let value = current
            current = current + 1
            return value
        }
        return null
    }
}
```

## Best Practices

1. **Keep protocols focused**: Each protocol should represent a single, coherent capability.

2. **Use protocol extensions for defaults**: Provide default implementations when possible to reduce boilerplate.

3. **Favor composition over inheritance**: Combine multiple small protocols rather than creating deep inheritance hierarchies.

4. **Use associated types for flexibility**: Associated types allow protocols to be more generic without requiring type parameters.

5. **Mark protocols public when needed**: Only expose protocols that are part of your public API.

6. **Use `Self` for fluent interfaces**: Return `Self` from methods to enable method chaining.

7. **Consider negative conformance**: Use `not Copyable` for types that manage resources and should not be copied.

## See Also

- [Generics](generics.md) - Generic programming with type parameters and constraints
- [Enums](enums.md) - Enumerated types that can also conform to protocols
- [Semantics](semantics.md) - Memory model and type system behavior
