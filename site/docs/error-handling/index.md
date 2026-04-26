# Error Handling

Kestrel handles "this might be missing" and "this might fail" through two stdlib enums: `Optional` and `Result`. There's no `null`, no exceptions — every absence and every failure is a value the compiler forces you to acknowledge.

## Optional vs Result

| Use | When |
|---|---|
| [`Optional[T]`](optional.md) | Value might not exist, and "missing" needs no explanation. (`lookup`, `firstOrNil`, `parse`.) |
| [`Result[T, E]`](result.md) | Value might not exist, and the *reason* matters. (`readFile`, `parseConfig`, `connect`.) |

If you find yourself returning `Result[T, ()]` because there's no useful error, you wanted `Optional[T]`. If you find yourself returning `Optional[T]` and the caller has to guess why, you wanted `Result[T, E]`.

## Try Operator

The `try` operator unwraps a `Result`. If it's `.Ok(value)`, you get the value. If it's `.Err(e)`, the surrounding function returns that error early.

```swift
func loadUser(id: Int) -> Result[User, Error] {
    let row = try database.fetch(id)       // returns early if .Err
    let parsed = try User.parse(row)        // returns early if .Err
    .Ok(parsed)
}
```

Without `try`, the same code would have nested `match` statements three levels deep. With `try`, the happy path stays linear.

`try` only works inside a function whose return type is `Result`-shaped (or compatible). Mixing it with `Optional` requires an explicit conversion — usually a `match` or a stdlib helper.

## Optional Promotion

When the expected type is `Optional[T]`, a value of type `T` is automatically promoted to `Optional.Some(value)`:

```swift
func cached(key: String) -> Optional[String] {
    "hello"   // promoted to .Some("hello") — no need to write it explicitly
}
```

This is a small affordance that keeps `Optional`-returning functions readable. It only happens at *return position* and other sites where the target type is unambiguously `Optional`.

## Subpages

- [Optional](optional.md) — values that may be absent
- [Result](result.md) — values that may carry an error

---

[← Pattern Matching](../enums/pattern-matching.md) · [↑ The Kestrel Language](../index.md) · [Optional →](optional.md)
