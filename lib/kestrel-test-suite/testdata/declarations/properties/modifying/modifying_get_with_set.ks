// test: execution
// stdlib: true
// backends: cranelift,llvm

// `modifying get` + `set`: compound assignment and mutating calls go
// read → modify → write back, and the read still counts.
module Test

import std.numeric.(Int64)
import std.collections.(Array)

struct Tracked {
    var raw: Int64
    var items: Array[Int64]
    var reads: Int64

    var value: Int64 {
        modifying get {
            self.reads += 1;
            self.raw
        }
        set { self.raw = newValue; }
    }

    var list: Array[Int64] {
        modifying get {
            self.reads += 1;
            self.items.clone()
        }
        set { self.items = newValue; }
    }

    subscript(i: Int64) -> Int64 {
        modifying get {
            self.reads += 1;
            self.items(i)
        }
        set { self.items(i) = newValue; }
    }
}

@main
func main() -> lang.i64 {
    var t = Tracked(raw: 1, items: [5, 6], reads: 0);
    t.value = 10;
    t.value += 5;
    if t.value != 15 { return 1; }
    t.list.append(7);
    if t.items.count != 3 { return 2; }
    t(0) += 1;
    if t(0) != 6 { return 3; }
    if t.reads != 5 { return 4; }
    0
}
