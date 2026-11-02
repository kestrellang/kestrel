// test: execution
// stdlib: true
// backends: cranelift,llvm

// A computed member declared `T throws E` is a throwing place. `get`
// returns `Result[T, E]`, `ref` lends `Result[&T, E]`, and `mutating ref`
// lends `Result[&mutating T, E]`, written through with an `.Ok(&mutating v)`
// binder. `try` reads it in a throwing function.
module Test

import std.numeric.(Int64)
import std.collections.(Array)

enum BadIndex {
    case Empty
    case OutOfRange
}

struct Bag {
    var items: Array[Int64]

    var top: Int64 throws BadIndex {
        get {
            if self.items.count == 0 { throw BadIndex.Empty; }
            .Ok(self.items(at: self.items.count - 1))
        }
    }

    var first: Int64 throws BadIndex {
        ref {
            if self.items.count == 0 { throw BadIndex.Empty; }
            let r = &self.items(at: 0);
            .Ok(r)
        }
        mutating ref {
            if self.items.count == 0 { throw BadIndex.Empty; }
            let r = &mutating self.items(at: 0);
            .Ok(r)
        }
    }

    subscript(checked i: Int64) -> Int64 throws BadIndex {
        ref {
            if i < 0 or i >= self.items.count { throw BadIndex.OutOfRange; }
            let r = &self.items(at: i);
            .Ok(r)
        }
        mutating ref {
            if i < 0 or i >= self.items.count { throw BadIndex.OutOfRange; }
            let r = &mutating self.items(at: i);
            .Ok(r)
        }
    }
}

func sum(b: Bag) -> Int64 throws BadIndex {
    let a = try b.top;
    let c = try b(checked: 0);
    .Ok(a + c)
}

@main
func main() -> lang.i64 {
    var b = Bag(items: [4, 5]);
    match sum(b) {
        .Ok(v) => { if v != 9 { return 1; } },
        .Err(_) => { return 2; },
    }
    match b.first {
        .Ok(v) => { if v != 4 { return 3; } },
        .Err(_) => { return 4; },
    }
    if let .Ok(&mutating v) = b.first { v += 10; }
    if b.items(at: 0) != 14 { return 5; }
    if let .Ok(&mutating v) = b(checked: 1) { v = 50; }
    if b.items(at: 1) != 50 { return 6; }
    if let .Ok(&mutating v) = b(checked: 7) { v = 0; return 7; }

    let empty = Bag(items: []);
    match sum(empty) {
        .Ok(_) => { return 8; },
        .Err(.Empty) => {},
        .Err(.OutOfRange) => { return 9; },
    }
    0
}
