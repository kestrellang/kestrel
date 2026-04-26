# Methods

Methods are functions attached to a type. They live in `extend` blocks, take an implicit `self`, and come in three flavors: instance, `mutating`, and `static`.

## Instance methods

```swift
struct Rectangle {
    let width: Int
    let height: Int
}

extend Rectangle {
    func area() -> Int {
        self.width * self.height
    }

    func describe() -> String {
        "\(self.width)×\(self.height) (area \(self.area()))"
    }
}

let r = Rectangle(width: 3, height: 4)
r.area()      // 12
r.describe()  // "3×4 (area 12)"
```

`self` refers to the instance the method is called on. Methods can call other methods on `self` without repeating the type.

## Mutating methods

A method that writes to a `var` field must be marked `mutating`:

```swift
struct Stack {
    var items: [Int]
}

extend Stack {
    mutating func push(value: Int) {
        self.items.append(value)
    }

    mutating func pop() -> Optional[Int] {
        self.items.removeLast()
    }
}

var s = Stack(items: [])
s.push(1)
s.push(2)
let top = s.pop()   // Optional.Some(2)
```

The caller has to hold the struct via `var`. A `let s` would refuse `s.push(1)`.

## Static methods

Static methods don't need an instance. Use them for constructors, factories, and utilities that belong to the type:

```swift
extend Rectangle {
    static func square(side: Int) -> Rectangle {
        Rectangle(width: side, height: side)
    }
}

let s = Rectangle.square(side: 5)
```

Call them on the type, not an instance.

## Method lookup

When you call `obj.foo()`, Kestrel looks for `foo` first as an instance method on the type, then on any protocol the type conforms to (with default-method fallback). See [Protocols → Default Methods](../protocols/default-methods.md).

---

[← Fields](fields.md) · [↑ Structs](index.md) · [Initializers →](initializers.md)
