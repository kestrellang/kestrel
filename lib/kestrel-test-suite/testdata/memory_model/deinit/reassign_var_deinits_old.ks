// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;
public var last_deinited_id: Int64 = 0;

struct Resource: not Copyable {
    var id: Int64
    deinit {
        deinit_count = deinit_count + 1;
        last_deinited_id = self.id;
    }
}

func main() -> lang.i64 {
    var x = Resource(id: 1);
    x = Resource(id: 2);

    // Reassignment deinited old value (id=1)
    if deinit_count != 1 { return 1; }
    if last_deinited_id != 1 { return 2; }
    0
}
