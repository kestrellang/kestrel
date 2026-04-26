// test: diagnostics
// stdlib: false
module Test
enum Node {
    case Leaf
    case Branch(data: Box) // ERROR: recursive enum requires `indirect`
}
struct Box {
    var node: Node
}
