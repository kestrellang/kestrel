// test: execution
// stdlib: true
// backends: cranelift,llvm

// A failable place (`var first?: T`, `subscript(...)? -> T`) may not exist:
// a `get` body returns `T?` (`return nil` for "no place"), and reads
// produce `T?`.
module Test

import std.numeric.(Int64)
import std.collections.(Array)

struct Stack {
    var items: Array[Int64]

    var top?: Int64 {
        get {
            if self.items.count == 0 { return null; }
            self.items(self.items.count - 1)
        }
    }

    subscript(checked i: Int64)? -> Int64 {
        get {
            if i < 0 or i >= self.items.count { return null; }
            self.items(i)
        }
    }
}

@main
func main() -> lang.i64 {
    let s = Stack(items: [1, 2, 3]);
    let e = Stack(items: []);
    if s.top != .Some(3) { return 1; }
    if e.top != null { return 2; }
    if s(checked: 1) != .Some(2) { return 3; }
    if s(checked: 9) != null { return 4; }
    match s(checked: 0) {
        .Some(v) => { if v != 1 { return 5; } },
        .None => { return 6; },
    }
    0
}
