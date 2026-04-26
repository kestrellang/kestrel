# Memory Model

Kestrel manages memory with **automatic reference counting** (ARC). Every value with heap-allocated storage carries a count of how many references point to it; when the count hits zero, the runtime frees the value and runs its [`deinit`](../structs/deinitializers.md) if one is defined.

You don't write `free`. You don't run a garbage collector. The compiler inserts retain/release calls at the boundaries where references are taken and dropped, and the runtime keeps the count honest.

## What's a reference vs a value

Most Kestrel types are **values** — copying a struct copies its fields. Two `Point` variables that hold `Point(x: 0, y: 0)` are independent; mutating one doesn't affect the other.

A few types — anything heap-backed, like `Array`, `String`, `Dict`, plus types you mark `class` (when classes are introduced) — are **references**. Copying the binding copies the reference; both names point to the same underlying storage. Writes through one are visible through the other.

For value types holding reference fields, the reference count tracks the inner storage:

```swift
struct PlayerList {
    var players: [Player]
}

let a = PlayerList(players: [Player(name: "Alice")])
let b = a   // copies the struct; players array is shared via ARC
```

`a` and `b` are independent `PlayerList` values, but they share the same `players` array under the hood. When the array is mutated through one (only possible if the holder is `var`), the change is visible through the other — until *copy on write* triggers, which it will when the array is about to grow.

## Copy on write

Mutable reference-typed collections in the stdlib (`Array`, `Dict`, `String`) are **copy-on-write**: a write triggers a copy of the underlying buffer if anyone else is holding a reference to it. The result is value-like semantics — your writes never accidentally bleed into someone else's binding — without paying the cost of an eager copy on every assignment.

You don't usually have to think about this. The mental model is "structs and stdlib collections behave like values" and the runtime makes it true cheaply.

## Reference cycles

ARC has one classic pitfall: if A references B and B references A, neither's count ever hits zero, and both leak. The compiler can't catch this in general — it requires `weak` or `unowned` references at the right edges to break the cycle. (The exact syntax for these is documented separately as language features land.)

The vast majority of Kestrel programs don't hit this. It comes up when you're building bidirectional graphs (parent ↔ child, observer ↔ subject) and worth being aware of, but isn't a daily concern.

## When `deinit` runs

`deinit` runs the moment the last reference goes away. That's deterministic — unlike a garbage collector, you can rely on cleanup happening at a predictable point in your code, which is what makes ARC suitable for managing scarce resources (file handles, sockets, locks). See [Structs → Deinitializers](../structs/deinitializers.md) for how to write one.

---

[← Type Inference](type-inference.md) · [↑ Concepts](index.md) · [Tooling →](../tooling/index.md)
