// Rosetta Code: Tree traversal
// https://rosettacode.org/wiki/Tree_traversal
module Main

indirect enum Tree {
    case Node(value: Int64, left: Tree, right: Tree)
    case Leaf
}

func node(v: Int64, l: Tree, r: Tree) -> Tree { .Node(value: v, left: l, right: r) }
func leaf(v: Int64) -> Tree { .Node(value: v, left: .Leaf, right: .Leaf) }

func preorder(t: Tree, mutating out: Array[Int64]) {
    if let .Node(v, l, r) = t {
        out.append(v);
        preorder(l, out);
        preorder(r, out);
    }
}

func inorder(t: Tree, mutating out: Array[Int64]) {
    if let .Node(v, l, r) = t {
        inorder(l, out);
        out.append(v);
        inorder(r, out);
    }
}

func postorder(t: Tree, mutating out: Array[Int64]) {
    if let .Node(v, l, r) = t {
        postorder(l, out);
        postorder(r, out);
        out.append(v);
    }
}

func levelOrder(t: Tree) -> Array[Int64] {
    var out = Array[Int64]();
    var queue = Deque[Tree]();
    queue.append(t);
    while let some next = queue.popFirst() {
        if let .Node(v, l, r) = next {
            out.append(v);
            queue.append(l);
            queue.append(r);
        }
    }
    out
}

@main
func main() {
    //         1
    //        / \
    //       2   3
    //      / \  /
    //     4   5 6
    //    /     / \
    //   7     8   9
    let tree = node(1,
        node(2, node(4, leaf(7), .Leaf), leaf(5)),
        node(3, node(6, leaf(8), leaf(9)), .Leaf));
    var pre = Array[Int64]();
    preorder(tree, pre);
    var ino = Array[Int64]();
    inorder(tree, ino);
    var post = Array[Int64]();
    postorder(tree, post);
    println("preorder:    \(pre.joined(by: " "))");
    println("inorder:     \(ino.joined(by: " "))");
    println("postorder:   \(post.joined(by: " "))");
    println("level-order: \(levelOrder(tree).joined(by: " "))");
}
