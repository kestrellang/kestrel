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

@main
func main() -> lang.i64 {
    var i: Int64 = 0;
    while i < 6 {
        let r = Resource(id: i);
        if i % 2 == 0 {
            consume(r);
        } else {
        }
        i = i + 1;
    }
    // All 6 deinited: 3 by consume's scope exit, 3 by loop body scope exit
    if deinit_count != 6 { return 1; }
    0
}
