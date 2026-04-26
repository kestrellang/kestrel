# Methods

A method is a function attached to a type. You call it on an instance with dot syntax, and `self` refers to that instance.

## Instance methods

Methods live in `extend` blocks rather than inside the struct definition:

```swift
struct Circle {
    let radius: Float
}

extend Circle {
    func area() -> Float {
        3.14159 * self.radius * self.radius
    }
}

let c = Circle(radius: 2.0)
let a = c.area()
```

`self` refers to the instance the method is called on. You don't declare it as a parameter — it's implicit.

## Mutating methods

A method that writes to a `var` field must be marked `mutating`:

```swift
struct Counter {
    var value: Int
}

extend Counter {
    mutating func increment() {
        self.value = self.value + 1
    }
}

var c = Counter(value: 0)
c.increment()   // c.value is now 1
```

The caller has to hold the instance via `var` — same rule as `mutating` parameters. See [Access Modes](access-modes.md).

## Static methods

A method that doesn't need an instance — usually a constructor or a utility — is `static`:

```swift
extend Point {
    static func origin() -> Point {
        Point(x: 0, y: 0)
    }
}

let p = Point.origin()
```

Call it on the type, not an instance.

## Methods are still functions

A method is a function with an implicit first parameter. They can have labels, return values, generics, and constraints just like any other function:

```swift
extend Vector {
    func dot[T](with other: T) -> Float where T: VectorLike {
        // ...
    }
}

v1.dot(with: v2)
```

For the broader function story (labels, access modes, closures), see the [Functions overview](index.md). For struct-side method coverage including initializers and computed properties, see [Structs](../structs/index.md).

---

[← Access Modes](access-modes.md) · [↑ Functions](index.md) · [Closures →](closures.md)
