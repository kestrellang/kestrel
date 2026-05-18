// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Inner: not Copyable {
    var id: Int64
    deinit {
        deinit_count = deinit_count + 1;
    }
}

struct Outer: not Copyable {
    var a: Inner
    var b: Inner

    init(fail_after_a fail: Bool)? {
        self.a = Inner(id: 1);
        if fail { return null; }
        self.b = Inner(id: 2);
    }
}

func test_success() {
    let result = Outer(fail_after_a: false);
    // Both inners alive, deinited at scope exit
}

func main() -> lang.i64 {
    // Fail after a: only Inner(id:1) deinited by partial drop
    deinit_count = 0;
    let result = Outer(fail_after_a: true);
    if deinit_count != 1 { return 1; }

    // Succeed: both inners deinited at scope exit
    deinit_count = 0;
    test_success();
    if deinit_count != 2 { return 2; }

    0
}
