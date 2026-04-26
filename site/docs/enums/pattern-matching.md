# Pattern Matching

`match` is how you take an enum apart. Each arm pairs a pattern with an expression, and the first matching arm runs.

## Basic match

```swift
match status {
    .Active => "running",
    .Paused => "halted",
    .Stopped => "off"
}
```

`match` is an expression — it produces a value, so you can use it on the right of `let`.

## Destructuring payloads

Bind payload fields by writing names in the pattern:

```swift
match shape {
    .Circle(radius) => Float.pi * radius * radius,
    .Rectangle(width, height) => width * height,
    .Point => 0.0
}
```

The names `radius`, `width`, `height` are *new bindings* introduced by the pattern. They're in scope only inside that arm.

Use `_` to ignore a payload:

```swift
match result {
    .Ok(value) => use(value),
    .Err(_) => fallback()
}
```

## Guards

Add a `where`-style guard with `if` to refine a match further:

```swift
match score {
    n if n > 90 => "excellent",
    n if n > 70 => "good",
    n if n > 50 => "passing",
    _ => "needs work"
}
```

The first arm whose pattern *and* guard both succeed runs. If a pattern matches but the guard fails, control falls through to the next arm.

## Nested patterns

Patterns can nest — destructure a payload that itself contains an enum:

```swift
match envelope {
    .Letter(.Urgent(message)) => alert(message),
    .Letter(.Normal(message)) => inbox.add(message),
    .Empty => {}
}
```

This is what makes recursive enums (like the `Spell.Combo(a, b)` in the [Wizard Duel tour](../tour/wizard-duel.md)) ergonomic — you destructure as deep as you need in one place.

## `if let`

When you only care about one variant, `if let` is shorter than a full `match`:

```swift
if let .Some(user) = lookup(id) {
    greet(user)
} else {
    println("not found")
}
```

`user` is in scope inside the `if` block. The `else` is optional.

## Exhaustiveness

The compiler verifies every variant is covered. Adding a new case to an enum will light up every existing `match` until you handle it. Treat that as a feature: it's how the compiler keeps your call sites honest as the data model evolves.

---

[← Enums](index.md) · [↑ Enums](index.md) · [Error Handling →](../error-handling/index.md)
