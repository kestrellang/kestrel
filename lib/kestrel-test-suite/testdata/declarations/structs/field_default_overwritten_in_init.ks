// test: execution
// stdlib: true
// backends: cranelift,llvm

// Every explicit init stores each field's `= expr` default before its body
// runs, so an init that assigns the field replaces the default (dropping
// it), and one that doesn't keeps it. Covers droppable fields, a generic
// struct and a failable init.
module Test

import std.numeric.(Int64)
import std.collections.(Array)

struct S {
    var tags: Array[Int64] = [1, 2, 3]
    var n: Int64 = 7

    init() { }
    init(tags t: Array[Int64]) { self.tags = t; }
    init(n x: Int64)? {
        if x < 0 { return null; }
        self.n = x;
    }
}

struct Box[T] {
    var item: T
    var count: Int64 = 1

    init(item i: T) { self.item = i; }
}

@main
func main() -> lang.i64 {
    let a = S();
    if a.tags.count != 3 { return 1; }
    if a.n != 7 { return 2; }
    let b = S(tags: [9]);
    if b.tags.count != 1 { return 3; }
    if b.n != 7 { return 4; }
    if let .Some(c) = S(n: 4) {
        if c.n != 4 { return 5; }
        if c.tags.count != 3 { return 6; }
    } else {
        return 7;
    }
    if let .Some(_) = S(n: -1) { return 8; }
    let x = Box(item: [5, 6]);
    if x.count != 1 { return 9; }
    0
}
