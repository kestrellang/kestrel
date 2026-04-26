# Computed Variables

A computed variable looks like a field but doesn't store anything. Its value comes from a `get` block, and optionally a `set` block lets it accept assignment.

## Read-only

```swift
struct Circle {
    var radius: Float

    var diameter: Float {
        get {
            self.radius * 2.0
        }
    }
}

let c = Circle(radius: 3.0)
c.diameter   // 6.0 — recomputed each access
```

`diameter` looks like a field at the call site, but every read calls the getter. Use computed variables when the value is cheap to derive and you'd rather not duplicate it as state.

## Read-write

A `set` block accepts a new value (named `newValue`) and updates the underlying state:

```swift
struct Temperature {
    var celsius: Float

    var fahrenheit: Float {
        get {
            self.celsius * 9.0 / 5.0 + 32.0
        }
        set {
            self.celsius = (newValue - 32.0) * 5.0 / 9.0
        }
    }
}

var t = Temperature(celsius: 0.0)
t.fahrenheit       // 32.0
t.fahrenheit = 212.0
t.celsius          // 100.0
```

`newValue` is implicit — you don't declare it as a parameter.

## When to use one

Computed variables shine when the value is conceptually a property of the type but mechanically derivable from other fields. They keep the call site clean (`circle.diameter` reads better than `circle.diameter()`) and avoid the bug class where a stored field drifts out of sync with the source-of-truth fields.

If the computation is expensive or has side effects, prefer a method — the parentheses warn the reader that it costs something.

---

[← Deinitializers](deinitializers.md) · [↑ Structs](index.md) · [Subscripts →](subscripts.md)
