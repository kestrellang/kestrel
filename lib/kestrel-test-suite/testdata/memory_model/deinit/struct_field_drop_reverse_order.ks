// test: execution
// stdlib: true
// expect-exit: 0

// #181: a struct's fields drop in REVERSE declaration order
// (docs/memory-model/drop-semantics.md), and the whole-value drop path must
// agree with the init-failure partial-drop path. `second` (last declared)
// drops before `first`.
module Test

import std.numeric.Int64

public var log: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    deinit { log = log * 10 + self.id; }
}

struct Container: not Copyable {
    var first: Res
    var second: Res
    deinit { log = log * 10 + 9; }
}

func build() {
    let c = Container(first: Res(id: 1), second: Res(id: 2));
    // Drop order: Container.deinit (9), then second (2), then first (1) => 921
}

@main
func main() -> lang.i64 {
    build();
    if log != 921 { return 1; }
    0
}
