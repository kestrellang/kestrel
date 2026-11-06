// test: execution
// stdlib: true
// backends: cranelift,llvm

// `&mutating self.items(i)` inside a `mutating` method: a subscript on
// a `var` field of a mutable `self` is a mutable place. It used to be
// rejected as "a `&mutating` borrow through a shared reference" (E210),
// because the method-call-shaped field subscript was classified by its
// `&T` read type.
module Test

import std.numeric.(Int64)
import std.collections.(Array)

struct Bag {
    var items: Array[Int64]

    mutating func firstMut() -> Optional[&mutating Int64] {
        if self.items.count == 0 { return null; }
        let r = &mutating self.items(0);
        .Some(r)
    }
}

@main
func main() -> lang.i64 {
    var b = Bag(items: [4, 5]);
    let q = b.firstMut();
    match q {
        .Some(e) => { e += 10; },
        .None => {},
    }
    if b.items(0) != 14 { return 1; }
    0
}
