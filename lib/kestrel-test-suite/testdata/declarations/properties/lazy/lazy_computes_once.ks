// test: execution
// stdlib: true
// backends: cranelift,llvm

// A `lazy var` runs its initializer on the first read only, can read
// `self`, starts empty in memberwise and explicit inits, and an assignment
// replaces the cached value without running the initializer.
module Test

import std.numeric.(Int64)
import std.collections.(Array)

struct Report {
    var base: Int64
    var runs: Int64

    lazy var total: Int64 = self.compute()

    mutating func compute() -> Int64 {
        self.runs += 1;
        self.base * 10
    }
}

struct Holder[T] {
    var seed: T
    lazy var copies: Array[T] = [self.seed, self.seed]

    init(seed: T) { self.seed = seed; }
}

@main
func main() -> lang.i64 {
    var r = Report(base: 4, runs: 0);
    if r.runs != 0 { return 1; }
    if r.total != 40 { return 2; }
    if r.total != 40 { return 3; }
    if r.runs != 1 { return 4; }
    r.base = 9;
    // Already cached: the change to `base` is not seen.
    if r.total != 40 { return 5; }

    var fresh = Report(base: 2, runs: 0);
    fresh.total = 7;
    if fresh.total != 7 { return 6; }
    if fresh.runs != 0 { return 7; }
    fresh.total += 1;
    if fresh.total != 8 { return 8; }

    var h = Holder[Int64](3);
    if h.copies.count != 2 { return 9; }
    h.copies.append(5);
    if h.copies.count != 3 { return 10; }
    0
}
