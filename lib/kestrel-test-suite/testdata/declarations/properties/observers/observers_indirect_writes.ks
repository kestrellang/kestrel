// test: execution
// stdlib: true
// backends: cranelift,llvm

// Writes that reach an observed field indirectly still run its observers:
// a subscript store or `+=` on the field, and a protocol `{ get set }`
// requirement the field witnesses.
module Test

import std.numeric.(Int64)
import std.collections.(Array)

protocol HasValue {
    var value: Int64 { get set }
}

struct Obs: HasValue {
    var hits: Int64
    var items: Array[Int64] {
        didSet { self.hits += 1; }
    }
    var value: Int64 {
        didSet { self.hits += 10; }
    }
}

func setIt[T](mutating t: T) where T: HasValue { t.value = 7; }

@main
func main() -> lang.i64 {
    var o = Obs(hits: 0, items: [1, 2], value: 0);
    o.items(at: 0) = 5;
    if o.items(at: 0) != 5 { return 1; }
    if o.hits != 1 { return 2; }
    setIt(o);
    if o.value != 7 { return 3; }
    if o.hits != 11 { return 4; }
    o.items(at: 1) += 3;
    if o.items(at: 1) != 5 { return 5; }
    if o.hits != 12 { return 6; }
    0
}
