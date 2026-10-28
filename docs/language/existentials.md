# Existential Types (`any P`)

An existential type, written `any P`, holds a value of *some* type that conforms to `P`, where the concrete type is only known at runtime. One `Array[any Shape]` can hold circles and squares side by side. Calls go through a function table chosen when the value was stored.

Compare `some P` ([opaque types](opaque-types.md)): there the compiler still knows the concrete type and there is no runtime cost. `any P` erases the type, so it costs a box and an indirect call, and in return lets one variable or collection hold different types over time.

```kestrel
protocol Shape {
    func area() -> Int64
}

struct Circle { var r: Int64; }
extend Circle: Shape {
    public func area() -> Int64 { 3 * self.r * self.r }
}

struct Square { var side: Int64; }
extend Square: Shape {
    public func area() -> Int64 { self.side * self.side }
}

let shapes: Array[any Shape] = [Circle(r: 1), Square(side: 2)];
var total: Int64 = 0;
for s in shapes { total = total + s.area(); }
```

A value becomes an existential wherever one is expected: a `Circle` passed for `any Shape` is boxed automatically. A bare protocol is not a type (`let s: Shape` is error E144); write `any Shape` or `some Shape`.

## Boxes

An existential always lives in a box, and the box is part of the type:

| Written | Box | Copying | Mutating through it | Consuming through it |
|---|---|---|---|---|
| `any P` | `RcBox` (the default) | Shares one payload (refcount) | Yes, every copy sees it | No |
| `any[CowBox] P` | `CowBox` | Shares until a write, then forks | Yes, forks first when shared | No |
| `any[OwnedBox] P` | `OwnedBox` | Only when `P: Cloneable` | Yes | Yes |

- **`any P` has reference semantics.** Copies share one payload, so a `mutating` call through one copy is visible through every other copy:

  ```kestrel
  var a: any Counter = Tally(n: 0);
  var b = a;          // same payload
  b.increment();
  a.value()           // 1
  ```

  Use `any[CowBox] P` for value semantics; it needs `P: Cloneable`, because a write forks by cloning the payload.
- **`any[OwnedBox] P` is the single-owner box.** It is the box to use for move-only payloads you want to give up later, and the only stdlib box that allows `consuming` requirements and `consuming as?`.
- **Boxes are not interconverted.** `any P` and `any[OwnedBox] P` are different types (E147). To move a payload into another box, get it out first with `as?`.
- **Your own box.** Any type with one `not Sized` type parameter that conforms to `ExistentialBox` and stores its payload behind a `Pointer` can be used as `any[B] P` (E402 explains what's missing otherwise).

## Composition and associated types

- `any P and Q` holds a value conforming to both protocols. It converts to `any P` or `any Q` (an *upcast*), as does `any R` when `R` refines `P`.
- `any Iterator[Item = Int64]` pins an associated type. Requirements that mention `Item` become callable, and the existential conforms to `Iterator` itself, so it works with `for`-`in` and generic code. A payload whose `Item` differs is rejected (E151).

## What you can call

A requirement can be called through `any P` (it is *dispatchable*) unless it:

- is `static` or an initializer (there is no value to dispatch on),
- is generic (a table can't hold every instantiation),
- uses `Self` outside the receiver, as `Equatable`'s `==` does, or
- mentions an associated type the existential leaves unbound.

Calling one of these is E145, which names the reason. Protocol extension methods work on any existential. `mutating` requirements need a box that grants mutation, and `consuming` ones need `ConsumingIndirection` (E148 suggests `any[OwnedBox] P`).

**`any P` conforms to `P`** when every requirement is dispatchable and the box grants the access each one needs. That lets you pass an existential to generic code (`func f[T](x: T) where T: P`, or a `some P` parameter). E146 names the requirement that blocks it.

On an existential, the protocol's members are found before the box's own: `RcBox.isUnique()` can't hide a requirement of the same name.

## Downcasting with `as?`

`x as? T` asks whether the payload is a `T`. It borrows the payload in place, so the result is `Optional[&T]`:

```kestrel
func radiusOf(s: any Shape) -> Int64 {
    guard let .Some(c) = s as? Circle else { return -1 }
    c.r
}
```

`consuming x as? T` gives the payload up instead. It returns `Result[T, any[B] P]`: on a hit the payload moves out and the box is freed; on a miss you get the existential back untouched.

```kestrel
let a: any[OwnedBox] Shape = Circle(r: 2);
match consuming a as? Circle {
    .Ok(c) => use(c),
    .Err(back) => use(back.area()),
}
```

It needs a box that can give up its payload (`ConsumingIndirection`); with the default `RcBox` it is E148.

`T` must be a concrete, sized type. A cast to a type that can't conform to the existential's protocols always fails, which is the E149 warning.

## Type tests with `is`

`x is T` tests the payload's type without binding it. It is typed like the literal `true`: `Bool` by default, or any `ExpressibleByBoolLiteral` type the context asks for. `x is not T` is the negation.

```kestrel
var circles: Int64 = 0;
for s in shapes { if s is Circle { circles = circles + 1; } }
let squares = shapes.filter(where: { (s) in s is Square });
```

`is` doesn't narrow `x` inside the branch (Kestrel has no flow typing); use an `is` pattern for that. The left side must be an existential: a concrete type is known at compile time, and a generic parameter is rejected (both E152).

## `is` patterns

| Pattern | Meaning |
|---|---|
| `is Circle` | The payload is a `Circle` |
| `c is Circle` | ...and `c` borrows it as `&Circle` |
| `is Rect { w, .. }` | ...and destructures it as a struct pattern |
| `r is Rect { w, .. }` | Both |
| `x @ is Circle` | `x` binds the whole existential |

They work in `match`, `if let`, `guard let` and `while let`. See [pattern matching](pattern-matching.md#type-test-patterns) for the rules.

## Errors

| Code | Meaning |
|---|---|
| E144 | A bare protocol used as a type |
| E145 | A requirement that can't be called through an existential |
| E146 | `any P` doesn't conform to `P` |
| E147 | Converting between existentials in different boxes |
| E148 | The box doesn't allow the access (`mutating`, `consuming`, `consuming as?`) |
| E149 | Warning: a cast or `is` test that always fails |
| E151 | A payload whose pinned associated type differs |
| E152 | `is` on a value that isn't an existential |
| E153 | `var` in an `is` pattern |
| E402 | A type that can't be used as a box |

## How it works

`any P` is sugar for `DefaultBox[rawany P]` and `any[B] P` for `B[rawany P]`, where `rawany P` is an unsized type with no spelling of its own. Pointers to it are two words: the payload's address and its function table. A table holds the payload's drop, size, alignment and clone, a pointer to each parent protocol's table, and one entry per dispatchable requirement. Tables are built once per box, payload type and protocol, so `as?` and `is` compare table addresses. The design is in [`docs/plans/existentials/`](../plans/existentials/existentials-design.md).
