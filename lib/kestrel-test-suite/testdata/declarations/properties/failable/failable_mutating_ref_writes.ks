// test: execution
// stdlib: true
// backends: cranelift,llvm

// A failable `mutating ref` accessor lends `Optional[&mutating T]`. A
// `&mutating` binder on its payload (`if let .Some(&mutating v) = b.first`)
// reads through it, so writes to `v` land in the element. Plain reads
// still go through `ref`.
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
        mutating ref {
            if self.items.count == 0 { return null; }
            let r = &mutating self.items(at: 0);
            .Some(r)
        }
    }

    // `get` serves reads, `mutating ref` serves `&mutating` binders.
    var last?: Int64 {
        get {
            if self.items.count == 0 { return null; }
            .Some(self.items(at: self.items.count - 1))
        }
        mutating ref {
            if self.items.count == 0 { return null; }
            let r = &mutating self.items(at: self.items.count - 1);
            .Some(r)
        }
    }

    subscript(checked i: Int64)? -> Int64 {
        ref {
            if i < 0 or i >= self.items.count { return null; }
            let r = &self.items(at: i);
            .Some(r)
        }
        mutating ref {
            if i < 0 or i >= self.items.count { return null; }
            let r = &mutating self.items(at: i);
            .Some(r)
        }
    }
}

@main
func main() -> lang.i64 {
    var b = Bag(items: [4, 5]);
    if let .Some(&mutating v) = b.first {
        v += 10;
    }
    if b.items(at: 0) != 14 { return 1; }

    match b(checked: 1) {
        .Some(&mutating v) => { v = 50; },
        .None => { return 2; },
    }
    if b.items(at: 1) != 50 { return 3; }

    // Out of range: nothing is written.
    if let .Some(&mutating v) = b(checked: 9) { v = 0; return 4; }

    // Plain reads still borrow through `ref`.
    match b.first {
        .Some(v) => { if v != 14 { return 5; } },
        .None => { return 6; },
    }

    if let .Some(&mutating v) = b.last { v += 1; }
    match b.last {
        .Some(v) => { if v != 51 { return 8; } },
        .None => { return 9; },
    }

    var empty = Bag(items: []);
    if let .Some(&mutating v) = empty.first { v = 1; return 7; }
    0
}
