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

func main() -> lang.i64 {
    var i: Int64 = 0;
    while i < 100 {
        consume(Resource(id: i));
        i = i + 1;
    }
    if deinit_count != 100 { return 1; }
    0
}
