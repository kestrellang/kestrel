# Generics

Generics allow types and functions to be parameterized over other types. A generic declaration introduces type parameters that behave like placeholders; the compiler resolves them at each use site so a single definition can work uniformly across many concrete types.

## Declaration Syntax

Type parameters are written in **square brackets** after the declaration name. This applies to every kind of generic declaration.

### Generic Struct

```kestrel
struct Box[T] {
    var value: T
}
```

### Generic Function

```kestrel
func identity[T](value: T) -> T {
    value
}
```

### Generic Protocol

```kestrel
protocol Container[T] {
    func add(item: T)
    func read() -> T
}
```

### Generic Enum

```kestrel
enum Result[T, E] {
    case ok(T)
    case err(E)
}
```

### Generic Type Alias

```kestrel
type Pair[A, B] = (A, B)
```

## Multiple Type Parameters

Separate type parameters with commas:

```kestrel
struct Map[K, V] {
    var entries: [(K, V)]
}

func pair[A, B](first: A, second: B) -> (A, B) {
    (first, second)
}
```

## Defaults

A type parameter may declare a default with `=`. Defaults let the caller omit that argument at the use site.

```kestrel
struct Map[K, V = String] { }

let a: Map[Int, Bool]   // K = Int, V = Bool
let b: Map[Int]         // K = Int, V = String (default)
```

Defaults must come **after** parameters without defaults:

```kestrel
struct Bad[T = Int, U] { }
// Error: type parameter with default must come after parameters without defaults
```

## Type Arguments

To instantiate a generic type, supply type arguments in square brackets:

```kestrel
let b: Box[Int]
let m: Map[String, Int]
let r: Result[Int, String]
```

Nested instantiations are written the same way:

```kestrel
let nested: Box[Option[Int]]
let matrix: Array[Array[Float64]]
```

### Type Arguments on Paths

Type arguments can appear on any segment of a path that names a generic:

```kestrel
Pointer[UInt8].nullPointer()
Array[Int].empty()
```

## Where Clauses

A `where` clause adds constraints on type parameters. It lives on the **declaration**, after the signature and before the body.

### Basic Bound

```kestrel
struct Set[T] where T: Hashable {
    var items: [T]
}
```

### Multiple Bounds on One Parameter

Use `and` to combine bounds on the same parameter:

```kestrel
struct SortedSet[T] where T: Hashable and Comparable { }
```

### Multiple Parameters

Separate constraints on different parameters with commas:

```kestrel
func zip[A, B](first: [A], second: [B]) -> [(A, B)]
    where A: Copyable, B: Copyable { }
```

### Generic Bounds

Bounds may themselves be generic protocols:

```kestrel
struct Counter[T] where T: Iterator[Int] { }
```

### Where Clauses on Functions

```kestrel
func max[T](a: T, b: T) -> T where T: Comparable {
    if a.greaterThan(other: b) { a } else { b }
}
```

### Where Clauses on Extensions

Extensions can be constrained, making their members available only when the constraints are satisfied:

```kestrel
extend Array[T] where T: Equatable {
    func contains(item: T) -> Bool { ... }
}
```

### Associated-Type Equality

Where clauses can also constrain an associated type to be equal to some other type. This is how you express "this method is only available when the iterator's element type is `Int`," and similar refinements.

```kestrel
extend Iterator where Item: Addable, Item.Output = Item {
    func sum() -> Item { ... }
}
```

## The Self Type

Inside a protocol or extension, `Self` refers to the conforming type. It is implicitly a type parameter bound by the enclosing protocol:

```kestrel
protocol Cloneable {
    func clone() -> Self
}

struct Point: Cloneable {
    var x: Int
    var y: Int

    // Self = Point here
    func clone() -> Point { Point(x: x, y: y) }
}
```

## Variance

Kestrel type parameters are **invariant** by default. Variance is not exposed in the surface syntax — `Box[Derived]` is not a subtype of `Box[Base]` even when `Derived` conforms to `Base`. To express subtype-like relationships, use protocol conformance rather than variance.

## Scope

Type parameters are in scope throughout the declaration that introduces them:

- Struct: visible in every field type, initializer, and method body.
- Protocol: visible in every method requirement, associated-type default, and extension.
- Function: visible in parameter types, return type, and body.
- Extension: visible in every member defined in the extension.

A type parameter name shadows any outer type with the same name for the duration of its scope.

## Validation Rules

### Duplicate Parameter Names

```kestrel
struct Bad[T, T] { }
// Error: duplicate type parameter `T`
```

### Default Ordering

```kestrel
struct Bad[T = Int, U] { }
// Error: type parameter with default must come after parameters without defaults
```

### Arity

The number of type arguments at a use site must match the declaration (accounting for defaults):

```kestrel
struct Box[T] { }

let a: Box              // Error: expected 1 type argument, found 0
let b: Box[Int, Bool]   // Error: expected at most 1 type argument, found 2
```

### Type Arguments on Non-Generic Types

```kestrel
struct Plain { }

let p: Plain[Int]
// Error: type `Plain` does not take type arguments
```

### Bounds Must Be Protocols

A where-clause bound must name a protocol (possibly with its own type arguments):

```kestrel
struct Thing[T] where T: SomeStruct { }
// Error: `SomeStruct` is not a protocol
```

### Undeclared Parameters in Where Clauses

```kestrel
struct Thing[T] where U: Equatable { }
// Error: undeclared type parameter `U` in where clause
```

## Errors

| Scenario | Error Message |
|----------|---------------|
| Too few type arguments | "expected N type arguments, found M" |
| Too many type arguments | "expected at most N type arguments, found M" |
| Type arguments on non-generic type | "type `X` does not take type arguments" |
| Unknown type parameter | "cannot find type `T` in this scope" |
| Duplicate parameter name | "duplicate type parameter `T`" |
| Default before non-default | "type parameter with default must come after parameters without defaults" |
| Non-protocol bound | "`X` is not a protocol" |
| Unknown bound | "cannot find protocol `X`" |
| Undeclared in where clause | "undeclared type parameter `T` in where clause" |

## Grammar

```ebnf
type_parameter_list =
    "[" type_parameter ("," type_parameter)* "]"

type_parameter =
    identifier (":" type_bound_list)? ("=" type_expr)?

type_bound_list =
    type_expr ("and" type_expr)*

type_argument_list =
    "[" type_expr ("," type_expr)* "]"

where_clause =
    "where" where_constraint ("," where_constraint)*

where_constraint =
    | type_bound_constraint
    | type_equality_constraint

type_bound_constraint =
    identifier ":" type_bound_list

type_equality_constraint =
    type_path "=" type_expr
```

Generic declarations in the wider grammar:

```ebnf
struct_declaration =
    visibility? "struct" identifier type_parameter_list? conformance_list? where_clause? "{" struct_body "}"

function_declaration =
    visibility? "func" identifier type_parameter_list? parameter_list return_type? where_clause? function_body

protocol_declaration =
    visibility? "protocol" identifier type_parameter_list? protocol_inheritance? where_clause? "{" protocol_body "}"

type_alias_declaration =
    visibility? "type" identifier type_parameter_list? "=" type_expr

extension_declaration =
    "extend" type_expr conformance_list? where_clause? "{" extension_body "}"
```

## Examples

### Nested Generics

```kestrel
struct Node[T] {
    var value: T
    var children: [Node[T]]
}
```

### Generic Function with Constraints

```kestrel
func sort[T](items: [T]) -> [T] where T: Comparable {
    // ...
}

let sorted = sort(items: [3, 1, 2])
```

### Constrained Extension

```kestrel
extend Box[T] where T: Cloneable {
    func deepCopy() -> Box[T] {
        Box(value: self.value.clone())
    }
}
```

### Generic Protocol Conformance

```kestrel
protocol Container[T] {
    func add(item: T)
    func read() -> T
}

struct IntBag: Container[Int] {
    var items: [Int]

    func add(item: Int) { items.append(item) }
    func read() -> Int { items(0) }
}
```

### Multi-Parameter Default

```kestrel
protocol Multipliable[Rhs = Self] {
    func multiply(other: Rhs) -> Self
}

// No need to spell out Rhs = Number
struct Number: Multipliable { ... }
```

## Best Practices

1. **Prefer protocol bounds over concrete types** in function signatures — `func foo[T](x: T) where T: Equatable` is more reusable than `func foo(x: SpecificType)`.
2. **Use defaults for common cases** — `Map[K, V = String]` lets callers omit the value type when it's usually `String`.
3. **Keep bound lists small** — long `where` clauses are a sign that a helper protocol should be extracted.
4. **Name parameters meaningfully** — `Element`, `Key`, `Value` read better than `T`, `U`, `V` when the role is specific.
5. **Reach for associated types instead of type parameters** when a protocol's implementer should choose the type, not the caller.

## See Also

- [Protocols](protocols.md) — protocol declarations, associated types, and conformance
- [Extensions](extensions.md) — adding members and conformances to existing types
- [Types](types.md) — the full type system
- [Semantics](semantics.md) — memory model and runtime behavior
