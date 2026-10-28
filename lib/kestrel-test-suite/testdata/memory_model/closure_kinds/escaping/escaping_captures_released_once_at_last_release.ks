// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/closures.md — "Copy and Drop": an acyclic escaping environment
// drops its captures EXACTLY ONCE at the last release. A non-Copyable `Res` is
// moved into the environment; while any handle lives nothing is dropped, and
// when both handles die the captured `Res.deinit` runs once (not zero, not two).
module Test

import std.numeric.Int64

public var drops: Int64 = 0;
public var fail: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    func value() -> Int64 { self.id }
    deinit { drops = drops + 1; }
}

func makeReader() -> escaping () -> Int64 {
    let r = Res(id: 7);
    { r.value() }        // method call widens the capture to `r`: moved into the env
}

func exercise() {
    let read = makeReader();
    let alias = read;                              // retain: second handle
    if read() != 7 { fail = 1; return; }
    if alias() != 7 { fail = 2; return; }
    if drops != 0 { fail = 3; return; }            // environment still live
}                                                  // last release happens here

@main
func main() -> lang.i64 {
    exercise();
    if fail != 0 { return 1 }
    if drops != 1 { return 2 }   // captured resource deinitialized exactly once
    0
}
