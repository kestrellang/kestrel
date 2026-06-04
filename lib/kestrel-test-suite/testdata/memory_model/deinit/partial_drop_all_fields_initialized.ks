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

    init(fail_late fail_late: Int64)? {
        self.a = Resource(id: 1)
        self.b = Resource(id: 2)
        // Fail after all fields are initialized — both should be deinited
        if fail_late != 0 { return null }
    }
}

@main
func main() -> lang.i64 {
    let failed = TwoResources(fail_late: 1);
    match failed {
        .Some(_) => { return 1 },
        _ => {}
    }
    // Both resources should be deinited
    if deinit_count != 2 { return 2 }
    0
}
