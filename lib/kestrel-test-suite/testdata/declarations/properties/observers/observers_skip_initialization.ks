// test: execution
// stdlib: true
// backends: cranelift,llvm

// Observers never fire while the value is initialized (memberwise or an
// explicit `init`), and an observer's own write to its property stores
// directly instead of re-running the observers.
module Test

import std.numeric.(Int64)

struct Clamped {
    var changes: Int64

    var value: Int64 {
        didSet {
            self.changes += 1;
            if self.value > 10 { self.value = 10; }
        }
    }

    init(start: Int64) {
        self.changes = 0;
        self.value = start;
        self.value = start + 1;
    }
}

struct Box[T] {
    var writes: Int64
    var item: T {
        willSet { self.writes += 1; }
    }
}

@main
func main() -> lang.i64 {
    var c = Clamped(50);
    if c.changes != 0 { return 1; }
    if c.value != 51 { return 2; }
    c.value = 99;
    if c.value != 10 { return 3; }
    if c.changes != 1 { return 4; }

    var b = Box[Int64](writes: 0, item: 1);
    b.item = 2;
    b.item += 3;
    if b.item != 5 { return 5; }
    if b.writes != 2 { return 6; }
    0
}
