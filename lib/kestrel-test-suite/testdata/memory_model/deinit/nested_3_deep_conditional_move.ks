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
    let r = Resource(id: 1);

    let c1: Bool = true;
    let c2: Bool = true;
    let c3: Bool = false;

    if c1 {
        if c2 {
            if c3 {
                consume(r);
            } else {
            }
        } else {
        }
    } else {
    }
    // c3 is false so r was NOT moved; deinit fires at scope exit
}

func main() -> lang.i64 {
    test();
    if deinit_count != 1 { return 1; }
    0
}
