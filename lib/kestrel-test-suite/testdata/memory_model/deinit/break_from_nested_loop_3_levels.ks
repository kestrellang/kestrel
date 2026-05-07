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

func main() -> lang.i64 {
    let outer_r = Resource(id: 1);

    outer: loop {
        let mid_r = Resource(id: 2);

        loop {
            let inner_r = Resource(id: 3);
            break outer;
        }
    }

    // inner_r and mid_r deinited by break; outer_r still alive
    if deinit_count != 2 { return 1; }
    0
}
