# Subscripts

A subscript lets you call an instance like a function — `container(key)` — to index into it. It's how `Array`, `Dictionary`, and friends are wired up; you can wire your own types the same way.

```swift
struct Grid {
    var cells: [[Int]]

    subscript(row: Int, col: Int) -> Int {
        get {
            self.cells[row][col]
        }
        set {
            self.cells[row][col] = newValue
        }
    }
}

var g = Grid(cells: [[0, 0, 0], [0, 0, 0]])
g(row: 1, col: 2) = 7
let value = g(row: 1, col: 2)   // 7
```

A subscript declaration looks a lot like a computed variable: `get` and an optional `set` (with implicit `newValue`). Parameters are labeled and can be of any type.

## Multiple subscripts

You can declare more than one — different label sets pick different subscripts, like overloaded functions:

```swift
extend Grid {
    subscript(at point: Point) -> Int {
        get { self.cells[point.y][point.x] }
        set { self.cells[point.y][point.x] = newValue }
    }
}

g(at: Point(x: 2, y: 1))   // same cell, different syntax
```

## Read-only subscripts

Drop the `set` block to make the subscript read-only:

```swift
extend Grid {
    subscript(rowSum at row: Int) -> Int {
        get { self.cells[row].sum() }
    }
}
```

Assigning to a read-only subscript is a compile error.

**Note**: subscripts are called with parentheses (`obj(key)`), not square brackets — square brackets in Kestrel are reserved for type parameters.

---

[← Computed Variables](computed-variables.md) · [↑ Structs](index.md) · [Enums →](../enums/index.md)
