// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;
public var deinit_order: Int64 = 0;

struct Resource: not Copyable {
    var id: Int64
    deinit {
        deinit_count = deinit_count + 1;
        deinit_order = deinit_order * 10 + self.id;
    }
}

func identity(consuming r: Resource) -> Resource {
    return r;
}

func test() {
    let a = Resource(id: 1);
    let b = identity(Resource(id: 2));
    let c = Resource(id: 3);
    // Scope exit: c(3), b(2), a(1) in reverse declaration order => 321
}

@main
func main() -> lang.i64 {
    test();
    if deinit_order != 321 { return 1; }
    0
}
