# Initializers

An initializer constructs a new instance of a struct. Kestrel synthesizes one for you by default, but you can write your own when the default isn't enough.

## The default memberwise initializer

If you don't write an `init`, you get one for free that takes one labeled argument per field:

```swift
struct Point {
    let x: Int
    let y: Int
}

Point(x: 0, y: 0)
```

Field defaults make those parameters optional:

```swift
struct Config {
    var port: Int = 8080
    let strict: Bool
}

Config(strict: true)              // port defaults
Config(port: 9000, strict: false) // override
```

## Custom initializers

When the default doesn't fit — you want to compute a field, validate input, or pick from multiple inputs — write your own:

```swift
struct Range {
    let lower: Int
    let upper: Int

    init(from start: Int, length: Int) {
        self.lower = start
        self.upper = start + length
    }
}

let r = Range(from: 10, length: 5)
```

Inside `init`, every `let` and `var` field must be assigned exactly once before the initializer returns. The compiler checks this — you can't accidentally leave a field uninitialized.

## Multiple initializers

You can have several `init`s with different labels — same overloading rules as ordinary functions:

```swift
extend Point {
    init(at x: Int, y: Int) {
        self.x = x
        self.y = y
    }

    init(angle: Float, distance: Float) {
        self.x = Int(distance * Float.cos(angle))
        self.y = Int(distance * Float.sin(angle))
    }
}

Point(at: 3, 4)
Point(angle: 1.57, distance: 10.0)
```

## Failable construction

If construction can fail, return an `Optional` or `Result` from a static factory rather than throwing from `init`:

```swift
extend User {
    static func parse(json: String) -> Optional[User] {
        // ...
    }
}
```

This keeps the failure visible at the call site instead of hiding it behind `init`.

---

[← Methods](methods.md) · [↑ Structs](index.md) · [Deinitializers →](deinitializers.md)
