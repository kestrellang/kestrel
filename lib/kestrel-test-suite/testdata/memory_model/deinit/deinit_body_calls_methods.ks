// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

public var log: Int64 = 0;

struct Resource: not Copyable {
    var id: Int64
    var active: Bool

    func report() -> Int64 {
        if self.active { return self.id; }
        return 0;
    }

    deinit {
        log = self.report();
    }
}

func test() {
    let r = Resource(id: 77, active: true);
    // Scope exit: deinit fires, calls report(), sets log = 77
}

func main() -> lang.i64 {
    test();
    if log != 77 { return 1; }
    0
}
