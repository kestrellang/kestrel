// test: execution
// stdlib: true
// backends: cranelift,llvm

// A closure that writes an observed field runs its observers, like a
// direct write: the closure captures the receiver, not the field's raw
// storage.
module Test

import std.numeric.(Int64)

struct Counter {
    var hits: Int64
    var n: Int64 {
        didSet { self.hits += 1; }
    }
    var pos: (Int64, Int64) {
        willSet { self.hits += 100; }
    }

    mutating func bump() {
        run({ self.n = 5; });
        run({ self.n += 1; });
        run({ self.pos.0 = 9; });
    }
}

func run(mutating f: mutating () -> ()) { f(); }

@main
func main() -> lang.i64 {
    var c = Counter(hits: 0, n: 0, pos: (0, 0));
    c.bump();
    if c.n != 6 { return 1; }
    if c.pos.0 != 9 { return 2; }
    if c.hits != 102 { return 3; }

    var d = Counter(hits: 0, n: 0, pos: (0, 0));
    run({ d.n = 3; });
    if d.n != 3 { return 4; }
    if d.hits != 1 { return 5; }
    0
}
