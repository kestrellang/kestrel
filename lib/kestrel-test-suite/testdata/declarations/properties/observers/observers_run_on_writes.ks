// test: execution
// stdlib: true
// backends: cranelift,llvm

// `willSet` / `didSet` on a stored property: plain assignment, compound
// assignment, mutating calls and nested stored writes all run both
// observers once, in order, with `newValue` / `oldValue` bound.
module Test

import std.numeric.(Int64)
import std.collections.(Array)

struct Point {
    var x: Int64
    var y: Int64
}

struct Counter {
    var log: Array[Int64]

    var count: Int64 = 0 {
        willSet { self.log.append(newValue); }
        didSet { self.log.append(oldValue * 100); }
    }

    var items: Array[Int64] {
        didSet { self.log.append(-1); }
    }

    var pos: Point {
        willSet { self.log.append(-2); }
    }

    mutating func bump() {
        self.count += 1;
    }
}

@main
func main() -> lang.i64 {
    var c = Counter(log: [], count: 3, items: [], pos: Point(x: 0, y: 0));
    // Memberwise init stores directly: no observer ran.
    if c.log.count != 0 { return 1; }

    c.count = 5;
    // willSet(5), didSet(old 3)
    if c.log.count != 2 { return 2; }
    if c.log(at: 0) != 5 { return 3; }
    if c.log(at: 1) != 300 { return 4; }

    c.count += 2;
    if c.count != 7 { return 5; }
    if c.log(at: 2) != 7 { return 6; }
    if c.log(at: 3) != 500 { return 7; }

    c.bump();
    if c.count != 8 { return 8; }
    if c.log.count != 6 { return 9; }

    c.items.append(1);
    if c.items.count != 1 { return 10; }
    if c.log(at: 6) != -1 { return 11; }

    c.pos.x = 4;
    if c.pos.x != 4 { return 12; }
    if c.log(at: 7) != -2 { return 13; }
    if c.log.count != 8 { return 14; }
    0
}
