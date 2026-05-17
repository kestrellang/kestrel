// test: diagnostics
// stdlib: true

module Test

struct Tree[T]: Cloneable where T: Cloneable {
    let value: T
    let forest: Forest[T]

    func clone() -> Tree[T] {
        Tree(value: self.value.clone(), forest: self.forest.clone())
    }
}
struct Forest[T]: Cloneable where T: Cloneable {
    let trees: [Tree[T]]

    func clone() -> Forest[T] {
        Forest(trees: self.trees.clone())
    }
}
