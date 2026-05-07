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
    let cond: Bool = true;

    if cond {
        consume(x);
    } else {
    }

    // x was moved; reassign resets the moved flag
    x = Resource(id: 2);

    // id=1 deinited by consume's scope exit
    if deinit_count != 1 { return 1; }
    0
}
