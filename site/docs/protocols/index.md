# Protocols

A protocol is a contract a type can satisfy. Anywhere a function names a protocol as a parameter type, any conforming value works — without that function knowing the concrete type.

## A first protocol

```swift
protocol Drawable {
    func draw()
}

struct Circle {
    let radius: Float
}

extend Circle: Drawable {
    public func draw() {
        println("○ (r=\(self.radius))")
    }
}

struct Square {
    let side: Float
}

extend Square: Drawable {
    public func draw() {
        println("□ (\(self.side))")
    }
}

func render(items: [Drawable]) {
    for item in items {
        item.draw()
    }
}
```

`Drawable` is the contract. `Circle` and `Square` both conform via `extend`. `render` works on a list of *any* drawable thing — heterogeneous arrays included.

This is Kestrel's primary tool for abstraction. Anywhere you'd reach for inheritance in a class-based language, reach for a protocol here.

## What you'll find here

- [Defining](defining.md) — declaring a protocol with required methods, properties, associated types
- [Conformance](conformance.md) — making a type satisfy a protocol
- [Default Methods](default-methods.md) — fallback implementations the conforming type can override
- [Inheritance Rules](inheritance-rules.md) — protocols composed of other protocols
- [Extending](extending.md) — adding requirements to a protocol after the fact

## When to reach for one

- You're writing a function that doesn't care which concrete type it gets, only what it can do.
- You're writing a collection of things that share a capability but not a structure (`[Castable]`, `[Place]`).
- You want default behavior that types can opt into without re-implementing.

If the answer is "I want to share data layout," use a struct, not a protocol — protocols describe *behavior*, not *fields*.

---

[← Result](../error-handling/result.md) · [↑ The Kestrel Language](../index.md) · [Defining →](defining.md)
