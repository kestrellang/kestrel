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

    init(fail_early fail_early: Int64)? {
        // Fail before initializing any fields
        if fail_early != 0 { return null }
        self.a = Resource(id: 1)
        self.b = Resource(id: 2)
    }
}

@main
func main() -> lang.i64 {
    // Fail before any field is initialized — deinit_count should stay 0
    let failed = TwoResources(fail_early: 1);
    match failed {
        .Some(_) => { return 1 },
        _ => {}
    }
    if deinit_count != 0 { return 2 }
    0
}
