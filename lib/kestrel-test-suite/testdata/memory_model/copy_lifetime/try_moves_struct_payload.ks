// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// `try f()` and `match f() { .Ok(v) => .. }` bind the payload of an owned
// temporary: it is moved out, not cloned and then dropped with the temporary.
// Covers a plain struct whose clone is derived from its fields (pre-mono it
// reports Bitwise; only its drop shows it owns anything).
module Test

import std.numeric.Int64

public var clones: Int64 = 0;

struct Leaf: Cloneable {
    var id: Int64
    func clone() -> Leaf { clones = clones + 1; return Leaf(id: self.id); }
}

struct Node { var leaf: Leaf; var tag: String }

func make(id: Int64) -> Node throws String { .Ok(Node(leaf: Leaf(id: id), tag: "n")) }

func viaTry() -> Int64 throws String {
    let n = try make(3);
    .Ok(n.leaf.id)
}

func idOf(n: Node) -> Int64 { n.leaf.id }

func viaMatch() -> Int64 {
    match make(4) {
        .Ok(n) => idOf(n),
        .Err(_) => 0
    }
}

@main
func main() -> lang.i64 {
    let a = match viaTry() { .Ok(v) => v, .Err(_) => 0 };
    if a != 3 { return 1 };
    if viaMatch() != 4 { return 2 };
    if clones != 0 { return 3 };
    0
}
