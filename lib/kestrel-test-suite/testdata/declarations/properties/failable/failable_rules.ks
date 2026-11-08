// test: diagnostics
// stdlib: true

// A failable place may not exist, so it is computed and read-only: no
// stored form, no `set`, and assignment through it is rejected.
module Test

import std.numeric.(Int64)
import std.collections.(Array)

struct Stored {
    var x?: Int64 // ERROR(E637): a failable member needs a read accessor
}

struct WithSet {
    var items: Array[Int64]
    var top?: Int64 { // ERROR(E637): a failable place cannot have a `set` accessor
        get { null }
        set { }
    }
}

struct Stack {
    var items: Array[Int64]
    var top?: Int64 {
        get { null }
    }
    subscript(checked i: Int64)? -> Int64 {
        get { null }
    }
}

func main() {
    var s = Stack(items: []);
    s.top = .Some(1); // ERROR(E214): cannot assign to failable place 'top'
    s(checked: 0) = .Some(2); // ERROR(E214): cannot assign to failable place 'subscript'
    s.top += 1; // ERROR(E214): cannot assign to failable place 'top'
}
