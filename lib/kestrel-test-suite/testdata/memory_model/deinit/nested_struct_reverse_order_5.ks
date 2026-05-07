// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

public var deinit_order: Int64 = 0;

struct L1: not Copyable {
    var id: Int64
    deinit { deinit_order = deinit_order * 10 + 1; }
}

struct L2: not Copyable {
    var inner: L1
    deinit { deinit_order = deinit_order * 10 + 2; }
}

struct L3: not Copyable {
    var inner: L2
    deinit { deinit_order = deinit_order * 10 + 3; }
}

struct L4: not Copyable {
    var inner: L3
    deinit { deinit_order = deinit_order * 10 + 4; }
}

struct L5: not Copyable {
    var inner: L4
    deinit { deinit_order = deinit_order * 10 + 5; }
}

func test() {
    let x = L5(inner: L4(inner: L3(inner: L2(inner: L1(id: 99)))));
    // Deinit order: L5(5), L4(4), L3(3), L2(2), L1(1) => 54321
}

func main() -> lang.i64 {
    test();
    if deinit_order != 54321 { return 1; }
    0
}
