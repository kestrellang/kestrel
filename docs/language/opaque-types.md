# Opaque Types (`some P`)

An opaque return type, written `some P`, lets a function promise "I return *some specific* type that conforms to `P`" without naming it. The caller programs against the protocol; the implementation keeps the concrete type private and free to change. Unlike a boxed existential, an opaque type is a purely compile-time abstraction — the compiler always knows the real type underneath, so there is no boxing, no vtable, and no runtime cost.

## Returning `some P`

```kestrel
protocol Shape {
    func area() -> Int64
}

struct Circle: Shape {
    var radius: Int64;
    func area() -> Int64 { 3 * self.radius * self.radius }
    func diameter() -> Int64 { 2 * self.radius }
}

func makeShape() -> some Shape {
    Circle(radius: 5)
}
```

The function body decides the concrete type (`Circle`); callers only see `some Shape`:

```kestrel
let s = makeShape();
println("area: \(s.area())");   // OK — area() is a Shape requirement
s.diameter();                    // ERROR — diameter() is Circle-only, hidden
```

What callers **can** do with an opaque value:

- Call the protocol's requirements, its protocol-extension methods, and requirements inherited from superprotocols.
- Pass it to generic functions constrained on the protocol (`func measure[T](s: T) where T: Shape`).

What they **cannot** do is see through to the concrete type — no concrete-only members, no downcasting.

## One Concrete Type per Function

All return paths of a `some P` function must produce the *same* concrete type:

```kestrel
func pick(flag: Bool) -> some Shape {
    if flag { return Circle(radius: 1); }
    Square(side: 2)   // ERROR: type mismatch — conflicting concrete types
}
```

The identity of an opaque type is the function it came from (plus its generic arguments). Two calls to `makeShape()` yield values of the same (hidden) type; values from *different* functions are unrelated types even if both are `some Shape`. A function whose only return is a recursive call to itself (directly or mutually) cannot anchor a concrete type and is rejected ("circular opaque return type").

## Protocol Composition

Compose bounds with `and` (the same syntax as `where` clauses — not `&`):

```kestrel
func makeBoth() -> some Shape and Printable {
    Circle(radius: 2)
}

let b = makeBoth();
b.describe();   // from Printable
b.area();       // from Shape
```

## Move-Only Underliers: `and not Copyable`

A plain `some P` promises callers a `Copyable` value, so the concrete type
behind it must be duplicable. Hiding a move-only type behind `some P` is
rejected:

```kestrel
struct Token: not Copyable { let value: Int64; }

func makeShape() -> some Shape {
    Token(7)   // ERROR: opaque return type hides non-Copyable type 'Token';
               //        add 'and not Copyable' to the return type
}
```

Appending the negative bound `and not Copyable` lifts the requirement — the
concrete type *may* be move-only (a Copyable underlier is still allowed).
In exchange, callers must treat the opaque value as move-only: it moves on
assignment and re-use after a move is an error.

```kestrel
func makeShape() -> some Shape and not Copyable {
    Token(7)   // OK
}

let s = makeShape();
let t = s;     // moves
s.area();      // ERROR: use of moved value
```

The negative bound must come last, and only `Copyable` may be negated —
`some P and not Q` for other protocols is rejected (the same rule as
conformance lists). In parameter position, `some P and not Copyable`
desugars to a generic parameter with a `where T: not Copyable` bound.

## Opaque Returns from Generic Types

Methods of generic types may return `some P`, including when the concrete type involves the enclosing type's parameters:

```kestrel
struct Factory[T] where T: Counter {
    var proto: T;
    func makeOne() -> some Counter { self.proto }   // concrete type is T
}

let f = Factory(proto: Fixed(n: 5));
println("\(f.makeOne().value())");
```

Each instantiation (`Factory[Fixed]`, `Factory[Other]`) gets its own distinct opaque type, resolved independently.

## `some P` in Parameter Position

In a parameter, `some P` is shorthand for an ordinary generic parameter — nothing is hidden, it's the *caller* who picks the type:

```kestrel
func render(shape: some Shape) -> Int64 {
    shape.area()
}
// identical to: func render[T](shape: T) -> Int64 where T: Shape
```

## Opaque vs Generics

The two are mirror images of who chooses the concrete type:

| | Generic (`[T]`, param-position `some`) | Opaque return (`-> some P`) |
|---|---|---|
| Who picks the type | The **caller** | The **callee** (function body) |
| Caller's view | Fully concrete after instantiation | Only the protocol interface |
| Use case | Work with anything conforming | Hide an implementation type |

Both are resolved at compile time via monomorphization — neither costs anything at runtime. Use `-> some P` when the concrete return type is an implementation detail (builders, iterator pipelines, factory functions); use a generic when the caller should control the type.

## Where `some` Is Allowed

- **Function/method return types** — the opaque case.
- **Function/method parameters** — generic sugar.

Not allowed:

- **Struct/enum fields** — E466 (`opaque types can only appear in return position`). A field has no function body to pin the concrete type. This includes `some` nested anywhere in the field's type (`[some P]`, `(some P, Int64)`, `some P?`).
- **Nested in a returned function type** (`-> (some Shape) -> Int64`) — rejected (currently as a conformance error).
- **Type aliases, closure annotations, multiple `some` per return (`(some P, some Q)`), downcasting an opaque value** — out of scope in v1.

## Not Yet Supported

- **`some P` on `let`/`var` bindings** (type-restriction form: hide a binding's concrete type behind a protocol) — designed, not implemented.
- **Associated-type constraints on the bound** (`some Iterable[Element = Int64]`) — designed, not implemented.
- `some ConcreteStruct` (a non-protocol bound) is currently accepted silently instead of being diagnosed; don't rely on it.

## Diagnostics You May Hit

| Code | Meaning |
|---|---|
| E466 | `some` in a field type — opaque types are return-position only |
| — | `type mismatch` — two return paths produce different concrete types |
| — | `does not conform` — the concrete return type doesn't satisfy the declared protocol bound |
| — | `circular opaque return type` — the concrete type can only be inferred from recursive calls |
| — | `no member '...'` — attempt to use a concrete-type member through the opaque interface |

## See Also

- [Protocols](protocols.md) — bounds, protocol extensions, superprotocols
- [Generics](generics.md) — the caller-chosen counterpart
- [Extensions](extensions.md) — conditional conformances that let one instantiation of a generic type satisfy the bound
