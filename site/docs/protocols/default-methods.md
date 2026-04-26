# Default Methods

A protocol can supply a default implementation for its requirements. Any conforming type gets the default for free; types that want different behavior override it.

## Defaults via extension

Provide a default by writing the body in an `extend` block on the protocol:

```swift
protocol Greeter {
    func name() -> String
    func greet()
}

extend Greeter {
    public func greet() {
        println("Hello, \(self.name())!")
    }
}
```

Now any type that conforms to `Greeter` only has to implement `name`. `greet` comes from the default:

```swift
struct Cat {
    let nickname: String
}

extend Cat: Greeter {
    public func name() -> String { self.nickname }
    // greet is inherited
}

Cat(nickname: "Mittens").greet()   // "Hello, Mittens!"
```

## Overriding the default

A conforming type can supply its own implementation, which takes precedence:

```swift
struct Robot {
    let model: String
}

extend Robot: Greeter {
    public func name() -> String { self.model }

    public func greet() {
        println("INITIALIZING. UNIT \(self.model) ONLINE.")
    }
}
```

## Why defaults matter

Defaults are how protocols stay ergonomic as they grow. Adding a new requirement to an existing protocol normally breaks every conforming type — but if the new requirement has a default, existing conformances keep compiling and authors opt in to overriding when they need different behavior. It's the difference between a protocol that's safe to evolve and one that becomes a standing tax on every conforming type.

The flip side: a default that doesn't make sense for every type isn't really a default — it's a footgun. If three out of five conforming types want different behavior, make it a requirement instead.

---

[← Conformance](conformance.md) · [↑ Protocols](index.md) · [Inheritance Rules →](inheritance-rules.md)
