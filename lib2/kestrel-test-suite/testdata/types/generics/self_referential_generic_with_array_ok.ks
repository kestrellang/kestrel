// test: diagnostics
// stdlib: true

module Test

struct Node[T] {
    let value: T
    let children: [Node[T]]
}
