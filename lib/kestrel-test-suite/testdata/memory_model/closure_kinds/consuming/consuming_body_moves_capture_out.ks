// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/closures.md: "Because it runs at most once, its body is the one
// place allowed to move captures *out* — return them or pass them onward."
// E506 is lifted inside a `consuming` body, so returning the captured
// non-Copyable is legal and the resource is deinitialized exactly once.
module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    deinit { deinit_count = deinit_count + 1; }
}

func takeRes(consuming f: consuming () -> Res) -> Int64 {
    let r = f();
    r.id
}

@main
func main() -> lang.i64 {
    let a = Res(id: 7);
    // `a` is moved into the environment, then moved back out by the body.
    let got = takeRes({ () in a });
    if got != 7 { return 1; }
    if deinit_count != 1 { return 2; }
    0
}
