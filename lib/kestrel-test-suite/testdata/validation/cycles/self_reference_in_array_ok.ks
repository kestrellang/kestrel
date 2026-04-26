// test: diagnostics
// stdlib: true

module Main

struct TreeNode: Cloneable {
    let value: lang.i64
    let children: [TreeNode]

    func clone() -> TreeNode {
        TreeNode(value: self.value, children: self.children.clone())
    }
}
