// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Resource: not Copyable {
    var id: Int64

    consuming func destroy() {}

    deinit {
        deinit_count = deinit_count + 1;
    }
}

@main
func main() -> lang.i64 {
    let r = Resource(id: 1);
    let cond: Bool = true;

    if cond {
        r.destroy();
    } else {
    }

    if deinit_count != 1 { return 1; }
    0
}
