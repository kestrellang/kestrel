// test: diagnostics
// stdlib: true

module Main

struct Node: Cloneable {
    let children: [Node]

    func clone() -> Node {
        Node(children: self.children.clone())
    }
}
