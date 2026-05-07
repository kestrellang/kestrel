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
    let a = Resource(id: 1);
    let b = Resource(id: 2);

    let cond1: Bool = true;
    let cond2: Bool = false;

    if cond1 {
        consume(a);
    } else {
    }

    if cond2 {
        consume(b);
    } else {
    }

    // a was consumed (deinited in consume's scope), b still alive
    if deinit_count != 1 { return 1; }
    0
}
