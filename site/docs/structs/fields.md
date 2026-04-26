# Fields

A field is a stored value attached to a struct. Each field has a name, a type, and a mutability declaration.

## Declaring fields

```swift
struct Player {
    let name: String       // immutable after init
    var hp: Int            // mutable
    var inventory: [Item]  // mutable
}
```

`let` fields are set once, in the initializer, and can't change after. `var` fields can be reassigned anytime — by methods marked `mutating`, or by external code holding the struct via `var`.

## Defaults

Give a field a default value to make it optional in the initializer:

```swift
struct Config {
    var port: Int = 8080
    var host: String = "localhost"
    let strict: Bool       // no default — required
}

let c = Config(strict: true)              // port and host take defaults
let d = Config(port: 9000, strict: false) // override port, default host
```

Fields without defaults must be supplied at construction.

## Field access

Read a field with dot syntax:

```swift
let portInUse = c.port
```

Write to a `var` field — provided you hold the struct via `var`:

```swift
var c = Config(strict: true)
c.port = 9001   // ok
```

`let` fields can't be written even from inside a mutating method. They're sealed at the type level.

---

[← Structs](index.md) · [↑ Structs](index.md) · [Methods →](methods.md)
