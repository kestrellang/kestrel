# Optional

`Optional[T]` represents a value that might not be there. It's an enum:

```swift
enum Optional[T] {
    case Some(T)
    case None
}
```

There's no `null` in Kestrel. Anything that might be absent — a dictionary lookup, a parse, a "first matching" search — returns an `Optional[T]`, and the compiler forces the caller to handle both cases.

## Constructing

Most of the time, the stdlib hands you an `Optional` and you don't construct one yourself. When you do, write the case explicitly or rely on optional promotion:

```swift
let nothing: Optional[Int] = .None
let something: Optional[Int] = .Some(42)

func cached(key: String) -> Optional[String] {
    "hit"   // promoted to .Some("hit") at return
}
```

## Unwrapping with `if let`

The most common pattern:

```swift
if let .Some(user) = lookup(id) {
    greet(user)
} else {
    println("not found")
}
```

`user` is in scope inside the `if` block, already unwrapped.

## Unwrapping with `match`

When you want to handle both cases as expressions:

```swift
let label = match settings.get("theme") {
    .Some(value) => value,
    .None => "default"
}
```

## Mapping and chaining

`Optional` has helper methods for common cases:

```swift
let length = name.map { it.count() }
                 .unwrapOr(0)
```

`map` applies a function inside the `Some`; `unwrapOr` provides a fallback for `None`. Use these when you'd otherwise be writing a one-liner `match`.

## When to use Optional vs Result

`Optional` says *something or nothing*. `Result` says *something or a reason*. If the caller needs to know *why* something is missing, return [`Result`](result.md) — `Optional[T]` throws away that information.

---

[← Error Handling](index.md) · [↑ Error Handling](index.md) · [Result →](result.md)
