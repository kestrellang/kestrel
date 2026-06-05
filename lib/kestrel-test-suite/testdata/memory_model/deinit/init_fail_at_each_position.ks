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

struct FiveFields: not Copyable {
    var a: Resource
    var b: Resource
    var c: Resource
    var d: Resource
    var e: Resource

    init(fail_at fail_at: Int64)? {
        self.a = Resource(id: 1);
        if fail_at == 1 { return null; }
        self.b = Resource(id: 2);
        if fail_at == 2 { return null; }
        self.c = Resource(id: 3);
        if fail_at == 3 { return null; }
        self.d = Resource(id: 4);
        if fail_at == 4 { return null; }
        self.e = Resource(id: 5);
    }
}

func test_success() {
    let r = FiveFields(fail_at: 0);
    // All 5 fields initialized, deinited at scope exit
}

@main
func main() -> lang.i64 {
    // Fail at 1: only a initialized => 1 deinit from partial drop
    deinit_count = 0;
    let r1 = FiveFields(fail_at: 1);
    if deinit_count != 1 { return 1; }

    // Fail at 2: a, b initialized => 2 deinits
    deinit_count = 0;
    let r2 = FiveFields(fail_at: 2);
    if deinit_count != 2 { return 2; }

    // Fail at 3: a, b, c => 3 deinits
    deinit_count = 0;
    let r3 = FiveFields(fail_at: 3);
    if deinit_count != 3 { return 3; }

    // Fail at 4: a, b, c, d => 4 deinits
    deinit_count = 0;
    let r4 = FiveFields(fail_at: 4);
    if deinit_count != 4 { return 4; }

    // Success: all 5 initialized, deinited at helper scope exit
    deinit_count = 0;
    test_success();
    if deinit_count != 5 { return 5; }

    0
}
