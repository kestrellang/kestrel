// test: diagnostics
// stdlib: true

module Test

struct Node[T]: Cloneable where T: Cloneable {
    let value: T
    let children: [Node[T]]

    func clone() -> Node[T] {
        Node(value: self.value.clone(), children: self.children.clone())
    }
}
