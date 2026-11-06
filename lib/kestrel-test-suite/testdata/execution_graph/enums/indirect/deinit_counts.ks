// test: execution
// stdlib: true
// expect-exit: 0
//
// Dropping a tree drops every payload exactly once — those inline in
// `.Leaf` and those reached through the boxes of `.Node`.

module Test

import std.numeric.(Int64)

var drops: Int64 = 0;

struct Payload: not Copyable {
    var id: Int64
    deinit { drops = drops + 1; }
}

indirect enum Tree: not Copyable {
    case Leaf(Payload)
    case Node(Tree, Tree)
}

func build() -> Int64 {
    let tree = Tree.Node(
        .Leaf(Payload(id: 1)),
        .Node(.Leaf(Payload(id: 2)), .Leaf(Payload(id: 3)))
    );
    if drops != 0 { return 1 }
    0
}

@main
func main() -> Int64 {
    let r = build();
    if r != 0 { return r }
    if drops != 3 { return 10 + drops }
    0
}
