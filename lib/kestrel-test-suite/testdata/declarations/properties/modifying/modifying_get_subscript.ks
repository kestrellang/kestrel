// test: execution
// stdlib: true
// backends: cranelift,llvm

// `modifying get` on a subscript: an insert-on-read lookup.
module Test

import std.numeric.(Int64)

struct Counter {
    var reads: Int64

    subscript(i: Int64) -> Int64 {
        modifying get {
            self.reads += 1;
            i * 2
        }
    }
}

@main
func main() -> lang.i64 {
    var c = Counter(reads: 0);
    let a = c(3);
    let b = c(4);
    if a != 6 { return 1; }
    if b != 8 { return 2; }
    if c.reads != 2 { return 3; }
    0
}
