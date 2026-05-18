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
    var x = Resource(id: 1);
    let cond: Bool = false;

    if cond {
        consume(x);
    } else {
    }

    // x was NOT moved (cond=false). Reassign deinits old value.
    x = Resource(id: 2);

    if deinit_count != 1 { return 1; }
    0
}
