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

func test() {
    var x = Resource(id: 0);
    var i: Int64 = 1;
    while i <= 5 {
        x = Resource(id: i);
        i = i + 1;
    }
    // 5 reassigns deinited 5 old values; final (id=5) deinited at scope exit
}

@main
func main() -> lang.i64 {
    test();
    // 5 old values + 1 final scope exit = 6 total
    if deinit_count != 6 { return 1; }
    0
}
