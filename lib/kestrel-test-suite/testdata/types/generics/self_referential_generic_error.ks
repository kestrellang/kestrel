// test: diagnostics
// stdlib: false

module Test

struct Node[T] {
    let value: T
    let next: Node[T] // ERROR: cannot contain itself
}
