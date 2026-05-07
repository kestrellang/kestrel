// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Resource: not Copyable {
    var id: Int64
    deinit {
        deinit_count = deinit_count + 1;
    }
}

func test_return() -> Int64 {
    let outer = Resource(id: 1);

    var i: Int64 = 0;
    while i < 10 {
        let inner = Resource(id: 2);
        if i == 3 {
            return deinit_count;
        }
        i = i + 1;
    }
    return deinit_count;
}

func main() -> lang.i64 {
    let result = test_return();
    // Iterations 0,1,2: inner deinited at end of each = 3
    // Iteration 3: return evaluates deinit_count (=3), then deinits inner + outer
    if result != 3 { return 1; }
    // After return: total = 5 (3 + inner + outer)
    if deinit_count != 5 { return 2; }
    0
}
