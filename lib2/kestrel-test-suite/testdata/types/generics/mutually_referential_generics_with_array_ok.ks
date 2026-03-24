// test: diagnostics
// stdlib: true

module Test

struct Tree[T] {
    let value: T
    let forest: Forest[T]
}
struct Forest[T] {
    let trees: [Tree[T]]
}
