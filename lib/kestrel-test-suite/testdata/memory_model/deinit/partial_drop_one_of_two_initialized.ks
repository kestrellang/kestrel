// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Resource: not Copyable {
    var id: Int64
    deinit {
        deinit_count = deinit_count + 1
    }
}

struct TwoResources: not Copyable {
    var a: Resource
    var b: Resource

    init(fail_mid fail_mid: Int64)? {
        self.a = Resource(id: 1)
        // Fail after first field but before second
        if fail_mid != 0 { return null }
        self.b = Resource(id: 2)
    }
}

@main
func main() -> lang.i64 {
    let failed = TwoResources(fail_mid: 1);
    match failed {
        .Some(_) => { return 1 },
        _ => {}
    }
    // Only 'a' was initialized — only 1 deinit should fire
    if deinit_count != 1 { return 2 }
    0
}
