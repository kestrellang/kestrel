# Extensions

Extensions add methods, computed properties, subscripts, and protocol conformances to existing types without modifying the original type definition. They enable retroactive modeling - adding capabilities to types after they're defined, including types from other modules or the standard library.

## Syntax

### Basic Extension

The simplest form - adding methods to an existing type:

```kestrel
struct Point {
    var x: lang.i64
    var y: lang.i64
}

extend Point {
    func sum() -> lang.i64 {
        lang.i64_add(self.x, self.y)
    }
}
```

### Extension with Protocol Conformance

Extensions can make types conform to protocols:

```kestrel
protocol Hashable {
    func hash() -> lang.i64
}

extend Point: Hashable {
    func hash() -> lang.i64 {
        lang.i64_add(self.x, self.y)
    }
}
```

### Multiple Protocol Conformances

A single extension can add conformance to multiple protocols:

```kestrel
protocol Hashable {
    func hash() -> lang.i64
}

protocol Describable {
    func describe() -> lang.str
}

extend Point: Hashable, Describable {
    func hash() -> lang.i64 {
        lang.i64_add(self.x, self.y)
    }

    func describe() -> lang.str {
        "a point"
    }
}
```

### Protocol Extension

Extensions can extend protocols themselves, providing default implementations:

```kestrel
protocol Drawable {
    func draw()
}

extend Drawable {
    func drawTwice() {
        self.draw()
        self.draw()
    }
}
```

Any type conforming to `Drawable` automatically gains the `drawTwice()` method.

## Adding Members

Extensions can add various kinds of members to types:

### Methods

Instance and static methods can be added:

```kestrel
extend Point {
    // Instance method
    func distance(other: Point) -> lang.i64 {
        let dx = lang.i64_sub(self.x, other.x)
        let dy = lang.i64_sub(self.y, other.y)
        lang.i64_add(lang.i64_mul(dx, dx), lang.i64_mul(dy, dy))
    }

    // Static method
    static func origin() -> Point {
        Point(x: 0, y: 0)
    }
}
```

Usage:
```kestrel
let p1 = Point(x: 3, y: 4)
let p2 = Point(x: 6, y: 8)
let dist = p1.distance(p2)
let origin = Point.origin()
```

### Computed Properties

Extensions can add computed properties but not stored properties:

```kestrel
extend Point {
    var magnitude: lang.i64 {
        lang.i64_add(lang.i64_mul(self.x, self.x), lang.i64_mul(self.y, self.y))
    }
}
```

See [Computed Properties](computed-properties.md) for more details on computed property syntax.

### Subscripts

Extensions can add subscript operations:

```kestrel
extend Array[T] {
    public subscript(safe index: lang.i64) -> Optional[T] {
        get {
            if index >= 0 and index < self.count {
                .Some(self.storage.buffer(unchecked: index))
            } else {
                .None
            }
        }
    }
}
```

See [Subscripts](subscripts.md) for more details on subscript syntax.

### Associated Types

Extensions that add protocol conformance can provide associated type bindings:

```kestrel
protocol Factory {
    type Product
    func make() -> Product
}

struct Maker { }

extend Maker: Factory {
    type Product = lang.i64

    func make() -> lang.i64 {
        1
    }
}
```

## Generic Extensions

Extensions can work with generic types in several ways:

### Extending All Instantiations

Extend a generic type for all type parameter values:

```kestrel
struct Box[T] {
    var value: T
}

extend Box[T] {
    func read() -> T {
        self.value
    }
}
```

The method `read()` is available on `Box[lang.i64]`, `Box[lang.str]`, and all other instantiations.

### Specialized Extensions

Extend a generic type for specific type arguments:

```kestrel
extend Box[lang.i64] {
    func doubled() -> lang.i64 {
        lang.i64_mul(self.value, 2)
    }
}
```

The `doubled()` method is only available on `Box[lang.i64]`, not on other instantiations like `Box[lang.str]`.

### Mixed Type Parameters

Extensions can mix type parameters with concrete types:

```kestrel
struct Pair[T, U] {
    var first: T
    var second: U
}

extend Pair[T, lang.i64] {
    func getSecond() -> lang.i64 {
        self.second
    }
}
```

This extension applies to `Pair[lang.str, lang.i64]` and `Pair[lang.i1, lang.i64]`, but not to `Pair[lang.str, lang.str]`.

### Free Type Parameters from the Conformance RHS

When extending a non-generic type with a generic protocol, the protocol's type arguments can introduce free type parameters that the extension is generic over. The extension does not list them on the keyword — they're introduced wherever they appear on the conformance:

```kestrel
protocol ArrayIndex[T] {
    type Output
    func loadFrom(array: Array[T]) -> Output
}

// `T` is not on the LHS — `Int64` has no type parameters. The conformance
// `ArrayIndex[T]` introduces `T` as a free parameter on this extension.
// The conformance reads as "for all T, Int64 conforms to ArrayIndex[T]".
extend Int64: ArrayIndex[T] {
    type ArrayIndex[T].Output = T

    public func loadFrom(array: Array[T]) -> T {
        array(unchecked: self)
    }
}
```

A few notes:

- **Single-uppercase identifiers** (`T`, `U`, `E`, `K`, `V`, …) are recognized as free parameters when they appear in the conformance RHS and aren't already in scope. Multi-letter names like `Self` or `Int64` are *not* introduced — they're treated as references to existing types.
- **Top-level position only.** `extend Int64: Foo[Box[T]]` does not auto-introduce `T`; put `T` at a top-level conformance argument instead.
- **No keyword-side declaration.** Kestrel deliberately does not use a form like `extend[T] Int64: ArrayIndex[T]`. The free parameter is introduced where it's used; the conformance is what's generic, not the extension itself.
- **Constraints on free parameters** ride in the where clause: `extend Int64: ArrayIndex[T] where T: Hashable`.

The qualified-binding form `type ArrayIndex[T].Output = T` (rather than bare `type Output = T`) ties the binding to a specific protocol — important when one type conforms to multiple protocols that each define an `Output`. The `T` on the right-hand side refers to the same free parameter introduced on the conformance line.

## Conditional Conformance

Extensions can add protocol conformances with additional constraints using where clauses:

### Basic Where Clause

```kestrel
protocol Equatable {
    func equals(other: Self) -> lang.i1
}

extend Box[T] where T: Equatable {
    func hasSameValue(other: Box[T]) -> lang.i1 {
        self.value.equals(other.value)
    }
}
```

The `hasSameValue()` method is only available when `T` conforms to `Equatable`.

### Multiple Constraints

Where clauses can include multiple constraints:

```kestrel
protocol Comparable {
    func lessThan(other: Self) -> lang.i1
}

protocol Hashable {
    func hash() -> lang.i64
}

struct SortedBox[T] where T: Comparable {
    var value: T
}

extend SortedBox[T] where T: Hashable {
    func getHash() -> lang.i64 {
        self.value.hash()
    }
}
```

This extension requires both the inherited constraint (`T: Comparable` from `SortedBox`) and the additional constraint (`T: Hashable` from the extension).

### Constrained Protocol Extensions

Protocol extensions can be constrained to apply only when conforming types meet certain requirements:

```kestrel
protocol Sortable {
    func sort()
}

protocol Filterable {
    func filter()
}

extend Filterable where Self: Sortable {
    func filterAndSort() {
        self.filter()
        self.sort()
    }
}
```

The `filterAndSort()` method is only available on types that conform to both `Filterable` and `Sortable`.

## Extension Specialization

When multiple extensions could apply to a type, the most specific one wins:

### Concrete vs Generic

```kestrel
extend Box[T] {
    func describe() -> lang.str {
        "generic box"
    }
}

extend Box[lang.i64] {
    func describe() -> lang.str {
        "lang.i64 box"
    }
}

func test() {
    let box1 = Box[lang.str](value: "hello")
    box1.describe()  // "generic box"

    let box2 = Box[lang.i64](value: 42)
    box2.describe()  // "lang.i64 box" (more specific wins)
}
```

### Partially vs Fully Specialized

```kestrel
struct Pair[T, U] {
    var first: T
    var second: U
}

extend Pair[T, U] {
    func describe() -> lang.str { "generic pair" }
}

extend Pair[T, lang.i64] {
    func describe() -> lang.str { "half specialized" }
}

extend Pair[lang.i64, lang.i64] {
    func describe() -> lang.str { "fully specialized" }
}
```

Specificity is based on the number of concrete type arguments:
- `Pair[T, U]`: specificity 0
- `Pair[T, lang.i64]`: specificity 1
- `Pair[lang.i64, lang.i64]`: specificity 2

### Protocol Extension Constraints

For protocol extensions, more constrained extensions win:

```kestrel
protocol A { func methodA() }
protocol B { func methodB() }
protocol C { func methodC() }

// Specificity 1 (one constraint)
extend C where Self: A {
    func helper() { }
}

// Specificity 2 (two constraints) - wins when both apply
extend C where Self: A, Self: B {
    func helper() { }
}
```

## Multiple Extensions

A type can have multiple extensions, and they're all merged together:

```kestrel
extend Point {
    func sum() -> lang.i64 {
        lang.i64_add(self.x, self.y)
    }
}

extend Point {
    func product() -> lang.i64 {
        lang.i64_mul(self.x, self.y)
    }
}

func test() {
    let p = Point(x: 3, y: 4)
    p.sum()      // Available from first extension
    p.product()  // Available from second extension
}
```

### Separate Conformance

Protocol conformance can be declared in one extension and satisfied by methods in another:

```kestrel
protocol Hashable {
    func hash() -> lang.i64
}

extend Point {
    func hash() -> lang.i64 {
        lang.i64_add(self.x, self.y)
    }
}

extend Point: Hashable { }  // Satisfied by previous extension
```

## Protocol Extensions

Extensions on protocols provide default implementations that all conforming types inherit:

### Default Methods

```kestrel
protocol Drawable {
    func draw()
    func clear()
}

extend Drawable {
    func redraw() {
        self.clear()
        self.draw()
    }
}

struct Circle: Drawable {
    func draw() { }
    func clear() { }
}

func test() {
    let c = Circle()
    c.redraw()  // Available from protocol extension
}
```

### Calling Protocol Methods

Protocol extension methods can call required protocol methods:

```kestrel
protocol Printable {
    func print()
}

extend Printable {
    func printTwice() {
        self.print()
        self.print()
    }
}
```

### Associated Type Usage

Protocol extensions can use associated types in signatures:

```kestrel
protocol Container {
    type Element
    func add(item: Element)
}

extend Container {
    func addTwo(first: Element, second: Element) {
        self.add(first)
        self.add(second)
    }
}
```

### Inherited Associated Types

Protocol extensions can access associated types from parent protocols:

```kestrel
protocol Base {
    type Element
}

protocol Child: Base {
    func fetch() -> Element
}

extend Child {
    func fetchWithFallback(fallback: Element) -> Element {
        self.fetch()
    }
}
```

## Visibility

Extension members can have their own visibility modifiers:

```kestrel
extend Point {
    public func publicSum() -> lang.i64 {
        lang.i64_add(self.x, self.y)
    }

    private func internalHelper() -> lang.i64 {
        lang.i64_mul(self.x, self.y)
    }

    func doubleSum() -> lang.i64 {
        lang.i64_mul(self.internalHelper(), 2)
    }
}
```

## Using `Self`

Extension methods can use the `Self` type to refer to the extended type:

```kestrel
extend Point {
    func clone() -> Self {
        Point(x: self.x, y: self.y)
    }

    func add(other: Self) -> Self {
        Point(
            x: lang.i64_add(self.x, other.x),
            y: lang.i64_add(self.y, other.y)
        )
    }
}
```

## Limitations

### Cannot Add Stored Properties

Extensions can only add computed properties, not stored properties:

```kestrel
extend Point {
    var magnitude: lang.i64 {  // Valid - computed property
        lang.i64_add(lang.i64_mul(self.x, self.x), lang.i64_mul(self.y, self.y))
    }

    // Invalid - cannot add stored properties
    // var z: lang.i64  // ERROR
}
```

### Cannot Extend Primitives

Built-in primitive types cannot be extended:

```kestrel
// Invalid - cannot extend primitive types
extend lang.i64 {  // ERROR
    func doubled() -> lang.i64 {
        lang.i64_mul(self, 2)
    }
}
```

### Cannot Extend Type Aliases

Extensions must target the actual type, not type aliases:

```kestrel
struct Point {
    var x: lang.i64
    var y: lang.i64
}

type MyPoint = Point

// Invalid - cannot extend type alias
extend MyPoint {  // ERROR
    func foo() { }
}

// Valid - extend the actual type
extend Point {
    func foo() { }
}
```

### Cannot Override Existing Members

Extensions cannot redefine members that already exist on the type:

```kestrel
struct Point {
    var x: lang.i64
    var y: lang.i64

    func sum() -> lang.i64 {
        lang.i64_add(self.x, self.y)
    }
}

extend Point {
    // Invalid - sum() already exists
    func sum() -> lang.i64 {  // ERROR: duplicate
        0
    }
}
```

### Same Specificity Conflicts

Two extensions with the same specificity cannot define the same member:

```kestrel
extend Point {
    func foo() -> lang.i64 { 1 }
}

extend Point {
    func foo() -> lang.i64 { 2 }  // ERROR: duplicate
}
```

But different specificity levels can define the same member:

```kestrel
extend Box[T] {
    func describe() -> lang.str { "generic" }
}

extend Box[lang.i64] {
    func describe() -> lang.str { "lang.i64" }  // OK - more specific
}
```

## Where Allowed

Extensions can be applied to:

- **Structs** - both generic and non-generic
- **Enums** - both generic and non-generic
- **Protocols** - to provide default implementations

Extensions cannot be applied to:

- Primitive types (`lang.i64`, `lang.str`, etc.)
- Type aliases
- Function types
- Tuple types

## Examples

### Adding Convenience Methods

```kestrel
struct Array[T] {
    private var storage: Buffer[T]
    private var count: lang.i64
}

extend Array[T] {
    func isEmpty() -> lang.i1 {
        self.count == 0
    }

    func first() -> Optional[T] {
        if self.isEmpty() {
            .None
        } else {
            .Some(self(0))
        }
    }
}
```

### Retroactive Protocol Conformance

```kestrel
protocol Comparable {
    func lessThan(other: Self) -> lang.i1
}

struct Point {
    var x: lang.i64
    var y: lang.i64
}

// Add conformance after Point is defined
extend Point: Comparable {
    func lessThan(other: Point) -> lang.i1 {
        if self.x == other.x {
            self.y < other.y
        } else {
            self.x < other.x
        }
    }
}
```

### Protocol-Oriented Design

```kestrel
protocol Sortable {
    func sort()
}

protocol Filterable {
    func filter()
}

// Provide functionality when both protocols are satisfied
extend Filterable where Self: Sortable {
    func filterAndSort() {
        self.filter()
        self.sort()
    }
}

struct Data: Filterable, Sortable {
    func filter() { }
    func sort() { }
}

func test() {
    let d = Data()
    d.filterAndSort()  // Available from extension
}
```

### Type-Constrained Extensions

```kestrel
protocol Mapper {
    type Source
    func map(s: Source)
}

struct Box[T] {
    var value: T
}

extend Box[T] where T: Mapper {
    func doMap(s: T.Source) {
        self.value.map(s)
    }
}
```

### Specialized Conformance

```kestrel
protocol Printable {
    func print()
}

struct Container[T] {
    var value: T
}

// Only Container[lang.i64] conforms to Printable
extend Container[lang.i64]: Printable {
    func print() { }
}

func usePrintable(p: Printable) {
    p.print()
}

func main() {
    let c = Container(value: 42)
    usePrintable(c)  // OK - Container[lang.i64] conforms

    // let s = Container(value: "hello")
    // usePrintable(s)  // ERROR - Container[lang.str] doesn't conform
}
```

## Grammar

```
ExtensionDeclaration → EXTEND Type ConformanceList? WhereClause? ExtensionBody

Type → TyPath TypeArgumentList?

ConformanceList → COLON ConformanceItem (COMMA ConformanceItem)*

ConformanceItem → Type

WhereClause → WHERE Constraint (COMMA Constraint)*

Constraint → TypeParameter COLON Type
           | SelfType COLON Type
           | AssociatedTypePath COLON Type

ExtensionBody → LBRACE ExtensionMember* RBRACE

ExtensionMember → FunctionDeclaration
                | SubscriptDeclaration
                | InitializerDeclaration
                | TypeAliasDeclaration
                | ComputedPropertyDeclaration

TypeArgumentList → LBRACKET Type (COMMA Type)* RBRACKET
```

## Implementation Notes

### Parser

The parser handles:
1. `extend` keyword as declaration starter
2. Type expression (potentially with type arguments like `Box[T]`)
3. Optional conformance list after `:`
4. Optional where clause
5. Extension body with allowed member declarations

### Semantic Analysis

- **BUILD phase**: Creates `ExtensionSymbol` with syntax span
- **BIND phase**:
  - Resolves target type and creates `ExtensionTargetBehavior`
  - Resolves protocol conformances
  - Resolves where clause constraints
  - Registers extension in `ExtensionRegistry` for method lookup
- **Type checking**: Extension members are type-checked within the context of the target type

### Extension Registry

Extensions are stored in an `ExtensionRegistry` indexed by the target type's symbol ID. During member lookup:
1. Direct members of the type are checked first
2. Extensions are queried from the registry
3. Most specific applicable extension wins based on:
   - Type argument specificity (more concrete types = higher specificity)
   - Where clause constraint count (more constraints = higher specificity)

### Method Resolution

When resolving a method call on a value:
1. Check the type's own methods
2. Check applicable extensions in specificity order
3. Check protocol extensions from conforming protocols
4. Type's own methods override extension methods
5. More specific extensions override less specific ones

### Constraint Inheritance

Extensions on generic types inherit constraints from the target type. For `extend SortedBox[T] where T: Hashable`, the full constraint set includes:
- Inherited: `T: Comparable` (from `SortedBox` definition)
- Added: `T: Hashable` (from extension where clause)
