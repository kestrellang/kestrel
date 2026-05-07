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

func consume(consuming r: Resource) {}

func test() {
    let r00 = Resource(id: 0);
    let r01 = Resource(id: 1);
    let r02 = Resource(id: 2);
    let r03 = Resource(id: 3);
    let r04 = Resource(id: 4);
    let r05 = Resource(id: 5);
    let r06 = Resource(id: 6);
    let r07 = Resource(id: 7);
    let r08 = Resource(id: 8);
    let r09 = Resource(id: 9);
    let r10 = Resource(id: 10);
    let r11 = Resource(id: 11);
    let r12 = Resource(id: 12);
    let r13 = Resource(id: 13);
    let r14 = Resource(id: 14);
    let r15 = Resource(id: 15);
    let r16 = Resource(id: 16);
    let r17 = Resource(id: 17);
    let r18 = Resource(id: 18);
    let r19 = Resource(id: 19);

    // Move even-indexed (10 moves)
    consume(r00);
    consume(r02);
    consume(r04);
    consume(r06);
    consume(r08);
    consume(r10);
    consume(r12);
    consume(r14);
    consume(r16);
    consume(r18);
    // Odd-indexed deinited at scope exit (10 more)
}

func main() -> lang.i64 {
    test();
    // 10 consumed + 10 scope exit = 20
    if deinit_count != 20 { return 1; }
    0
}
