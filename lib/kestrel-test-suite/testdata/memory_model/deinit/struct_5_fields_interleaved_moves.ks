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
    let r1 = Resource(id: 1);
    let r2 = Resource(id: 2);
    let r3 = Resource(id: 3);
    let r4 = Resource(id: 4);
    let r5 = Resource(id: 5);

    consume(r1);
    consume(r3);
    consume(r5);
    // 3 deinits from consume; r2 and r4 deinited at scope exit
}

func main() -> lang.i64 {
    test();
    // 3 consumed + 2 scope exit = 5 total
    if deinit_count != 5 { return 1; }
    0
}
