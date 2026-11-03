// test: diagnostics
// stdlib: true

// A `&mutating` binder on a failable member's payload needs a mutable
// receiver and a `mutating ref` accessor to read through.
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
    var last?: Int64 {
        ref {
            if self.items.count == 0 { return null; }
            let r = &self.items(at: self.items.count - 1);
            .Some(r)
        }
    }
}

func main() {
    let fixed = Bag(items: [1]);
    if let .Some(&mutating v) = fixed.first { v = 2; } // ERROR(E210)

    var b = Bag(items: [1]);
    if let .Some(&mutating v) = b.last { v = 2; } // ERROR(E210)
}
