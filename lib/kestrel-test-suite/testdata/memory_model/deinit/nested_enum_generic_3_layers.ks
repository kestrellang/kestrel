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

enum Wrapper[T]: not Copyable {
    case Some(T)
    case None
}

func test() {
    let deep = Wrapper[Wrapper[Wrapper[Resource]]].Some(
        Wrapper[Wrapper[Resource]].Some(
            Wrapper[Resource].Some(
                Resource(id: 42)
            )
        )
    );
    // Scope exit recursively deinits through all 3 enum layers
}

@main
func main() -> lang.i64 {
    test();
    if deinit_count != 1 { return 1; }
    0
}
