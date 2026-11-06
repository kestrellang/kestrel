// test: execution
// stdlib: true
// backends: cranelift,llvm

// Only writes to the initializer's own `self` skip observers. Inside an
// init, writing another instance of the same type runs its observers, and
// an init declared in an extension skips them for its own `self` too.
module Test

import std.numeric.(Int64)

struct Gauge {
    var changes: Int64
    var level: Int64 {
        didSet { self.changes += 1; }
    }

    init(level l: Int64) {
        self.changes = 0;
        self.level = l;
    }

    // Copies `other`, then bumps `other`: the write to `other` is not
    // initialization, so its observer runs.
    init(mutating copying other: Gauge) {
        self.changes = 0;
        self.level = other.level;
        other.level = other.level + 1;
    }
}

extend Gauge {
    init(doubled n: Int64) {
        self.changes = 0;
        self.level = n;
        self.level = n * 2;
    }
}

@main
func main() -> lang.i64 {
    var a = Gauge(level: 1);
    if a.changes != 0 { return 1; }
    let b = Gauge(copying: a);
    if b.changes != 0 { return 2; }
    if a.level != 2 { return 3; }
    if a.changes != 1 { return 4; }

    let d = Gauge(doubled: 3);
    if d.level != 6 { return 5; }
    if d.changes != 0 { return 6; }
    0
}
