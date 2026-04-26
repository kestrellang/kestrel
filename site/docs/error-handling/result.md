# Result

`Result[T, E]` is a value that's either a success or a failure with a reason. It's an enum:

```swift
enum Result[T, E] {
    case Ok(T)
    case Err(E)
}
```

Use `Result` for functions that can fail in distinguishable ways — file I/O, parsing, validation, network calls. The error type `E` is whatever you want; usually an enum that captures the failure modes.

## A typical pattern

```swift
enum LoadError {
    case NotFound
    case PermissionDenied
    case Corrupt(reason: String)
}

func loadConfig(path: String) -> Result[Config, LoadError] {
    let bytes = match readFile(path) {
        .Some(b) => b,
        .None => return .Err(.NotFound)
    }
    // ...parse bytes, return .Ok(config) or .Err(.Corrupt(...))
}
```

The caller knows exactly what can go wrong because `LoadError` enumerates it. They can `match` on the `Err` to handle each kind specifically — or `try` to bubble it up.

## The `try` operator

`try` unwraps a `Result` inline. On `.Ok(value)`, it gives you the value; on `.Err(e)`, it returns from the surrounding function with that error.

```swift
func setup() -> Result[Server, LoadError] {
    let config = try loadConfig(path: "/etc/app.toml")
    let port = try parsePort(config.port)
    .Ok(Server(config: config, port: port))
}
```

This is the single most useful tool for keeping `Result`-using code readable. Without it you'd be nesting `match` blocks; with it the happy path stays linear and the failure path is implicit.

`try` requires the surrounding function's return type to be `Result`-shaped. The error types have to be compatible — if the inner `Err` type doesn't match the outer one, you'll need to map it first.

## Mapping and chaining

```swift
let port: Result[Int, LoadError] =
    loadConfig(path: "/etc/app.toml")
        .map { it.port }
        .mapErr { .Corrupt(reason: "\(it)") }
```

`map` transforms the `Ok`. `mapErr` transforms the `Err`. They keep the chain flat when you don't need full `try` propagation.

## When to use Result vs Optional

If the caller needs to know *why* something failed, use `Result`. If "missing" is the whole story (`Dict[key]`, `firstWhere`, `parse`), use [`Optional`](optional.md).

---

[← Optional](optional.md) · [↑ Error Handling](index.md) · [Protocols →](../protocols/index.md)
