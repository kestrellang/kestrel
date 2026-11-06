// test: execution
// stdlib: true
// backends: cranelift,llvm

// A `modifying get` subscript read on an observed field mutates the field,
// so it runs the observers like any other write.
module Test
import std.numeric.(Int64)
struct Counter {
    var n: Int64
    subscript(i: Int64) -> Int64 {
        modifying get { self.n += i; self.n }
    }
    mutating func bump() { self.n += 100; }
}
struct O {
    var hits: Int64
    var c: Counter {
        didSet { self.hits += 1; }
    }
}
@main
func main() -> lang.i64 {
    var o = O(hits: 0, c: Counter(n: 0));
    let x = o.c(5);
    if x != 5 { return 1; }
    if o.c.n != 5 { return 2; }
    if o.hits != 1 { return 3; }
    o.c.bump();
    if o.c.n != 105 { return 4; }
    if o.hits != 2 { return 5; }
    0
}
