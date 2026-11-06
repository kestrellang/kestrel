// test: execution
// stdlib: true
// expect-exit: 0
//
// Only recursive cases are boxed. `Leaf`'s three words stay inline, so the
// enum is as wide as them, while the boxed `Node` adds just its box. A
// non-recursive case's payload is dropped in place, exactly once.

module Test

import std.numeric.(Int64)
import std.memory.(Layout, OwnedBox)

var drops: Int64 = 0;

struct Wide: not Copyable {
    var a: Int64
    var b: Int64
    var c: Int64
    deinit { drops = drops + 1; }
}

indirect enum Tree: not Copyable {
    case Leaf(Wide)
    case Node(Tree, Tree)
}

func leafOnly() -> Int64 {
    let leaf = Tree.Leaf(Wide(a: 1, b: 2, c: 3));
    match leaf {
        .Leaf(w) => if w.a + w.b + w.c != 6 { return 1 },
        .Node(_, _) => return 2
    };
    0
}

@main
func main() -> Int64 {
    let wide = Layout.of[Wide]().size;
    let box = Layout.of[OwnedBox[(Tree, Tree)]]().size;
    let tree = Layout.of[Tree]().size;
    // Inline Leaf: at least its payload. Boxed Node: just the box. So the
    // enum is the wider of the two plus at most a word of tag.
    let widest = if wide > box { wide } else { box };
    if tree < wide { return 1 }
    if tree > widest + 8 { return 2 }
    let r = leafOnly();
    if r != 0 { return 10 + r }
    if drops != 1 { return 20 + drops }
    0
}
