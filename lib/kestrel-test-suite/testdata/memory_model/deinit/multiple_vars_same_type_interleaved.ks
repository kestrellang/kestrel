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
    let v1 = Resource(id: 1);
    let v2 = Resource(id: 2);
    let v3 = Resource(id: 3);
    let v4 = Resource(id: 4);

    consume(v1);
    consume(v3);

    // v1 and v3 deinited in consume; v2 and v4 still alive
    if deinit_count != 2 { return 1; }
    0
}
