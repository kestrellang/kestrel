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

struct Box[T]: not Copyable {
    var value: T
    deinit {
        deinit_count = deinit_count + 1;
    }
}

func test() {
    let b = Box[Resource](value: Resource(id: 1));
    // Scope exit: Box.deinit (count=1), then Resource.deinit (count=2)
}

@main
func main() -> lang.i64 {
    test();
    if deinit_count != 2 { return 1; }
    0
}
