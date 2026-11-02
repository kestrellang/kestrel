// test: execution
// stdlib: true
// backends: cranelift,llvm

// A failable `ref` accessor lends `Optional[&T]`: reads borrow the element
// instead of copying it.
module Test

import std.numeric.(Int64)
import std.collections.(Array)

struct Bag {
    var items: Array[Int64]

    var first?: Int64 {
        ref {
            if self.items.count == 0 { return null; }
            let r = &self.items(at: 0);
            .Some(r)
        }
    }

    subscript(checked i: Int64)? -> Int64 {
        ref {
            if i < 0 or i >= self.items.count { return null; }
            let r = &self.items(at: i);
            .Some(r)
        }
    }
}

@main
func main() -> lang.i64 {
    let b = Bag(items: [4, 5]);
    let none = Bag(items: []);
    match b.first {
        .Some(v) => { if v != 4 { return 1; } },
        .None => { return 2; },
    }
    match none.first {
        .Some(_) => { return 3; },
        .None => {},
    }
    match b(checked: 1) {
        .Some(v) => { if v != 5 { return 4; } },
        .None => { return 5; },
    }
    match b(checked: 2) {
        .Some(_) => { return 6; },
        .None => {},
    }
    0
}
