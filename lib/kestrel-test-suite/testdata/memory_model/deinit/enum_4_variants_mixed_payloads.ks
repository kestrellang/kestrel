// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Heavy: not Copyable {
    var id: Int64
    deinit {
        deinit_count = deinit_count + 1;
    }
}

struct Light: not Copyable {
    var id: Int64
    deinit {
        deinit_count = deinit_count + 1;
    }
}

enum Mixed: not Copyable {
    case HeavyCase(value: Heavy)
    case LightCase(value: Light)
    case CopyableCase(n: Int64)
    case EmptyCase
}

func test_heavy() {
    let a = Mixed.HeavyCase(value: Heavy(id: 1));
}

func test_light() {
    let b = Mixed.LightCase(value: Light(id: 2));
}

func test_copyable() {
    let c = Mixed.CopyableCase(n: 42);
}

func test_empty() {
    let d = Mixed.EmptyCase;
}

@main
func main() -> lang.i64 {
    deinit_count = 0;
    test_heavy();
    if deinit_count != 1 { return 1; }

    deinit_count = 0;
    test_light();
    if deinit_count != 1 { return 2; }

    deinit_count = 0;
    test_copyable();
    if deinit_count != 0 { return 3; }

    deinit_count = 0;
    test_empty();
    if deinit_count != 0 { return 4; }

    0
}
