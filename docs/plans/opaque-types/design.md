# Opaque Types (`some Protocol`) — Design Document

## Summary

Opaque types let a declaration hide a concrete type behind a protocol interface.
The caller sees `some P` and can only use `P`'s methods; the concrete type is
known to the compiler but not exposed. No boxing, no vtables, no runtime cost —
opaque types are a compile-time abstraction that evaporates before MIR.

```kestrel
func makeShape() -> some Shape {
    Circle(radius: 5)
}

let s = makeShape()
s.draw()       // OK — Shape method
s.radius       // ERROR — concrete type hidden
```

## Syntax

The keyword `some` precedes a protocol bound in type position:

```
some <ProtocolRef>
some <ProtocolRef> and <ProtocolRef>
some <ProtocolRef>[AssocType = ConcreteType]
some <ProtocolRef> and not Copyable
```

Composition uses `and`, matching existing where-clause syntax (`T: P and Q`).

## Categories

`some` has different semantics depending on where it appears. These collapse
into four mechanisms:

### Category 1: Generic sugar (parameter position)

**Syntax triggers:** `some P` in function/method/init/subscript parameter types.

**Semantics:** Desugars to an unnamed generic type parameter.

```kestrel
func draw(shape: some Drawable) { ... }
// desugars to:
func draw[__T0: Drawable](shape: __T0) { ... }
```

Each `some` in the parameter list creates a **separate** type parameter. Two
`some Drawable` params are independent types — they can accept different
concrete types.

```kestrel
func overlay(a: some Drawable, b: some Drawable) { ... }
// desugars to:
func overlay[__T0: Drawable, __T1: Drawable](a: __T0, b: __T1) { ... }
```

**Implementation:** Pure syntactic transformation in HIR lowering. No new type
system concepts. The synthesized type parameters are invisible to the user
(cannot be named, turbofished, or referenced).

### Category 2: Opaque return (return position)

**Syntax triggers:** `some P` in return types of functions, computed properties,
and subscripts that have a body.

**Semantics:** The function implementation chooses one concrete type. Callers see
only the protocol interface. All return paths must yield the same concrete type.

```kestrel
func makeShape() -> some Shape {
    Circle(radius: 5)    // concrete type: Circle
}

let s = makeShape()      // s: some Shape (actually Circle)
s.draw()                 // OK
s.radius                 // ERROR — not part of Shape
```

**Rules:**

- All return expressions must unify to a single concrete type.
- The concrete type must conform to the declared bound.
- Each distinct set of generic arguments produces a distinct opaque type.
- Opaque type identity = (function entity, generic arguments).

```kestrel
// ERROR: conflicting return types
func bad(flag: Bool) -> some Shape {
    if flag { return Circle() }
    return Square()   // Circle vs Square
}

// OK: per-generic-arg identity
func wrap[T: Printable](value: T) -> some Printable {
    Wrapper[T](value)  // concrete type is Wrapper[T]
}
// wrap[Int64]() and wrap[String]() are different opaque types
```

**Computed properties and subscripts** are syntactic variants — they desugar to
getter/setter functions, so they use the same mechanism.

```kestrel
var shape: some Shape { Circle(radius: 10) }
subscript(i: Int64) -> some Element { storage[i] }
```

**Structural positions** — `some` may appear nested inside a return type, but
limited to **one `some` per function** for v1:

```kestrel
func find() -> some Shape?          { ... }  // Optional[Opaque]
func items() -> Array[some Shape]   { ... }  // Array[Opaque]
func lazy() -> () -> some Shape     { ... }  // closure returning Opaque
```

`some` in the *parameter* position of a returned function type is illegal —
the caller would need to produce a value of the hidden type:

```kestrel
func f() -> (some Shape) -> Void    // ERROR
```

### Category 3: Associated type sugar (protocol requirements)

**Syntax triggers:** `some P` in return type or property type of a protocol
method/property declaration (no body).

**Semantics:** Desugars to an anonymous associated type.

```kestrel
protocol Factory {
    func make() -> some Shape
}
// desugars to:
protocol Factory {
    type __Make_Return: Shape
    func make() -> __Make_Return
}
```

Conforming types satisfy the associated type by providing a concrete
implementation:

```kestrel
struct CircleFactory: Factory {
    func make() -> some Shape {
        Circle(radius: 10)   // pins __Make_Return = Circle
    }
}
```

**Implementation:** Syntactic transformation during protocol analysis. The
anonymous associated type is generated and wired into the protocol's type
parameter list. Conforming types use Category 2 (opaque return) for their
implementations.

### Category 4: Type restriction (variable/property position)

**Syntax triggers:** `some P` in a `let`/`var` declaration or stored property
with an initializer.

**Semantics:** The concrete type is pinned by the initializer. Only the protocol
interface is visible on the variable.

```kestrel
let x: some Numeric = 42       // concrete type: Int64
x + 1                           // OK — Numeric has +
x.isEven                        // ERROR — Int64-specific
```

For `var`, reassignment is allowed but only to the same concrete type:

```kestrel
var x: some Numeric = 42
x = 100                         // OK — still Int64
x = 3.14                        // ERROR — Float64 is not Int64
```

**Stored properties** in structs pin one concrete type for all instances:

```kestrel
struct Player {
    let shape: some Shape = Circle(radius: 10)
}
// ALL Player instances have shape: Circle
// Player is NOT implicitly generic
```

A stored property with `some` **must** have a default initializer. If omitted,
all `init` methods must agree on the same concrete type. Conflicts are diagnosed.

**Implementation:** The compiler infers the concrete type from the initializer
and stores a `Type::Opaque` in the variable's type binding. Member resolution
delegates to the protocol. Before MIR lowering, the opaque type resolves to the
concrete type.

## Visibility Model: Nothing Leaks

Only what is explicitly declared after `some` is visible to callers. Nothing
about the concrete type leaks through.

| Visible to caller?                               | Answer                         |
| ------------------------------------------------ | ------------------------------ |
| Protocol methods                                 | Yes                            |
| Superprotocol conformances                        | Yes (part of the contract)     |
| Protocol extension methods                        | Yes (part of the interface)    |
| Constrained associated types (`some P[Item = X]`) | Yes — `Item` is concrete       |
| Unconstrained associated types                    | Opaque but consistent          |
| Other protocol conformances of concrete type      | No                             |
| Concrete type identity                            | No                             |

### Copyable follows language defaults

Kestrel types are `Copyable` by default. Opaque types follow the same rule:

- `some Shape` — concrete type must be `Copyable` (default).
- `some Shape and not Copyable` — concrete type may or may not be `Copyable`.

This is not "leaking" — it's the language-wide default applied consistently.

### Associated type visibility

Unconstrained associated types are opaque but consistent:

```kestrel
func items() -> some Iterable {
    [1, 2, 3]
}

let it = items()
let a = it.next()    // type: (some Iterable).Element — opaque
let b = it.next()    // same opaque type
a == b               // ERROR — Element not known to be Equatable
```

Constrained associated types are concrete:

```kestrel
func items() -> some Iterable[Element = Int64] {
    [1, 2, 3]
}

let it = items()
let a = it.next()    // type: Int64
a + 1                // OK
```

## Protocol Bound Specificity

### Generic protocol parameters — optional

```kestrel
some Container            // OK — generic params unconstrained (opaque)
some Container[Int64]     // OK — constrains generic param
```

### Associated type constraints — optional, named

```kestrel
some Iterator                        // OK — Element is opaque
some Iterator[Element = String]      // OK — Element is String
```

### Composition — `and` keyword

```kestrel
some Shape and Equatable
some Iterable[Element = Int64] and Printable
some Shape and not Copyable
```

### Type aliases — supported

```kestrel
type StringCollection = Collection[Element = String]
func f() -> some StringCollection { ... }   // works
```

Type aliases are resolved before evaluating `some`, so this works for free.

## Conformance Passing

An opaque type is a concrete (unnamed) type that conforms to its bound. It
satisfies generic constraints:

```kestrel
func take[T: Shape](x: T) { ... }
let s = makeShape()    // some Shape
take(s)                // OK — opaque type conforms to Shape
```

## Recursive Functions

The concrete type must be inferrable from at least one non-recursive return:

```kestrel
// OK — base case pins Int64
func f(n: Int64) -> some Numeric {
    if n == 0 { return 42 }
    return f(n - 1)
}

// ERROR — no concrete return, circular inference
func g() -> some Shape { g() }

// ERROR — mutual recursion, circular
func h() -> some Shape { k() }
func k() -> some Shape { h() }
```

## Type System Representation

### New TyKind variant

```rust
Opaque {
    origin: Entity,           // function/property that defines this
    bound: Entity,            // protocol entity
    bound_args: Vec<TyVar>,   // protocol generic args
    index: u32,               // for future multi-some (always 0 in v1)
}
```

### Member resolution

`Opaque{bound=P}` delegates member resolution to protocol `P`. This includes:
- Direct protocol methods
- Protocol extension methods
- Superprotocol methods

### Conformance

`Opaque{bound=P}` conforms to `P` and all of `P`'s superprotocols. Composition
(`some P and Q`) conforms to both `P` and `Q` and their superprotocols.

### Resolution

After type checking, all `Type::Opaque` instances are resolved to their stored
concrete types before MIR lowering. MIR and codegen never see opaque types.

For generic functions, the concrete type may still contain type parameters
(`Wrapper[T]`). These are substituted during monomorphization as usual.

## Diagnostics

| ID   | Message                                                  | When                                                |
| ---- | -------------------------------------------------------- | --------------------------------------------------- |
| E4xx | all returns must have the same concrete type             | conflicting return types across branches             |
| E4xx | concrete type `X` does not conform to `P`               | return type doesn't satisfy the bound                |
| E4xx | cannot infer concrete type for opaque return             | no non-recursive return path                         |
| E4xx | `some` is not allowed in this position                   | type alias, closure annotation, cast, where clause   |
| E4xx | cannot access `X` on opaque type `some P`               | using concrete-type-only members                     |
| E4xx | stored property with `some` type requires an initializer | `struct S { let x: some P }` with no default         |
| E4xx | conflicting concrete types across initializers           | two `init` methods pin different types                |
| E4xx | circular opaque type inference                           | mutual recursion with no concrete base case           |

## Out of Scope for v1

- Multiple `some` per return type (`(some P, some Q)`)
- `some` in closure type annotations
- `some` in type aliases (`type Foo = some P`)
- Downcasting opaque types (`value as? ConcreteType`)
- `any P` to `some P` implicit existential opening
- `some P and not Q` negative bounds (other than `not Copyable`)

## Implementation Phases

### Phase 1: Parser
Accept `some` before type references. New syntax node `SomeType` wrapping an
inner type reference with optional `and`-separated additional bounds.

### Phase 2: HIR / Binder — Category 1 (generic sugar)
Desugar `some P` in parameter position to synthetic type parameters with where
clauses.

### Phase 3: HIR / Binder — Category 3 (associated type sugar)
Desugar `some P` in protocol requirement positions to anonymous associated
types.

### Phase 4: Type inference — Category 2 (opaque return)
Add `TyKind::Opaque`. Handle two views of return type: internal (concrete,
inferred from body) and external (opaque, protocol-restricted). Add member
resolution, conformance checks, and coercion rules for opaque types.

### Phase 5: Type inference — Category 4 (type restriction)
Handle `some P` on variables and stored properties. Infer concrete type from
initializer, expose as opaque.

### Phase 6: Resolution
Resolve all `Type::Opaque` to concrete types before MIR lowering.

### Phase 7: Diagnostics and tests
Wire up error messages, write test cases for all categories and edge cases.
