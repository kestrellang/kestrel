// test: execution
// stdlib: true
// backends: cranelift,llvm

// Failable places compose with generics and with `modifying get`.
module Test

import std.numeric.(Int64)
import std.collections.(Array)

struct Queue[T] {
    var items: Array[T]
    var pops: Int64

    // Reading `next` pops the front element, if any.
    var next?: T {
        modifying get {
            if self.items.count == 0 { return null; }
            self.pops += 1;
            .Some(self.items.remove(at: 0))
        }
    }

    subscript(checked i: Int64)? -> T {
        ref {
            if i < 0 or i >= self.items.count { return null; }
            let r = &self.items(i);
            .Some(r)
        }
    }
}

@main
func main() -> lang.i64 {
    var q = Queue[Int64](items: [7, 8], pops: 0);
    match q(checked: 1) {
        .Some(v) => { if v != 8 { return 1; } },
        .None => { return 2; },
    }
    if q.next != .Some(7) { return 3; }
    if q.next != .Some(8) { return 4; }
    if q.next != null { return 5; }
    if q.pops != 2 { return 6; }
    0
}
