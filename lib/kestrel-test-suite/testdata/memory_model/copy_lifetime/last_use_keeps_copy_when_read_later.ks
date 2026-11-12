// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: 7 7\n7\n

// A read is only moved when nothing can read the local again. Each of these
// reads it later: plainly, and through a view closure created earlier. Both
// must keep the copy (one clone each) and still see the value.
module Test

import std.numeric.Int64

public var clones: Int64 = 0;

struct Leaf: Cloneable {
    var id: Int64
    func clone() -> Leaf { clones = clones + 1; return Leaf(id: self.id); }
}

struct Holder { var leaf: Leaf }

func readAfter() -> Int64 {
    let x = Leaf(id: 7);
    let h = Holder(leaf: x);
    println("\(h.leaf.id) \(x.id)");
    h.leaf.id
}

func readThroughClosure() -> Int64 {
    let x = Leaf(id: 7);
    let f = { x.id };
    let h = Holder(leaf: x);
    println("\(f())");
    h.leaf.id
}

@main
func main() -> lang.i64 {
    if readAfter() != 7 { return 1 };
    if readThroughClosure() != 7 { return 2 };
    if clones != 2 { return 3 };
    0
}
