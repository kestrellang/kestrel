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

struct Wrapper: not Copyable {
    var inner: Resource
    deinit {
        deinit_count = deinit_count + 1;
    }
}

func wrap(consuming r: Resource) -> Wrapper {
    return Wrapper(inner: r);
}

func consume(consuming w: Wrapper) {}

@main
func main() -> lang.i64 {
    consume(wrap(Resource(id: 1)));

    // Wrapper deinit + Resource deinit = 2
    if deinit_count != 2 { return 1; }
    0
}
