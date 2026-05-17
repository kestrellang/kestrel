// test: diagnostics
// stdlib: false

module Test

struct Tree[T] {
    let value: T
    let children: Forest[T] // ERROR: circular struct containment
}
struct Forest[T] {
    let trees: Tree[T]
}
