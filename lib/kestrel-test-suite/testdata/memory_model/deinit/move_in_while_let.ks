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

func consume(consuming r: Resource) {}

func produce(n: Int64) -> Resource? {
    if n > 0 {
        return .Some(Resource(id: n));
    }
    return .None;
}

func main() -> lang.i64 {
    var counter: Int64 = 3;
    while let .Some(v) = produce(counter) {
        consume(v);
        counter = counter - 1;
    }
    // 3 iterations: consume deinits each
    if deinit_count != 3 { return 1; }
    0
}
