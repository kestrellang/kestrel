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
    let choice: Int64 = 3;

    match choice {
        1 => { consume(r); },
        2 => { consume(r); },
        3 => {},
        _ => {}
    }
    // choice=3, r NOT moved, deinited at scope exit
}

@main
func main() -> lang.i64 {
    test();
    if deinit_count != 1 { return 1; }
    0
}
