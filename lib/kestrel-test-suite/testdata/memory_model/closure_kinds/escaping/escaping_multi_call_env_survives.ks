// test: execution
// stdlib: true
// expect-exit: 0

// Plan D5, "Escaping env parameter ownership" (docs/plans/closure-kinds/
// closure-kinds-plan.md): the env parameter of an escaping closure must be
// BORROWED/guaranteed (or a raw storage pointer), never `Consuming`/owned. An
// owned handle parameter would release once per call and free a multi-call
// environment after the FIRST call; projection inside the body goes through
// `sharedMutRef` semantics (borrow, never consume).
//
// Guard: three calls through one handle, then a call through a copied handle,
// then another through the original — all must observe the same live,
// accumulating environment. The captured resource carries a `deinit`, so the
// refcount discipline is checked from both ends: nothing is released while a
// handle lives, and exactly one release happens after the last one dies.
module Test

import std.numeric.Int64

public var releases: Int64 = 0;
public var fail: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    func value() -> Int64 { self.id }
    deinit { releases = releases + 1; }
}

func makeCounter() -> escaping () -> Int64 {
    var count = 0;
    let r = Res(id: 100);                    // method call widens to `r`: moved into the env
    { () in count = count + 1; count + r.value() }
}

func exercise() {
    let next = makeCounter();
    if next() != 101 { fail = 1; return; }   // call 1 through handle A
    if next() != 102 { fail = 2; return; }   // call 2 — env NOT freed by call 1
    if next() != 103 { fail = 3; return; }   // call 3
    let alias = next;                        // retain: second handle, one environment
    if alias() != 104 { fail = 4; return; }  // call through the copied handle
    if next() != 105 { fail = 5; return; }   // original handle still sees the shared state
    if releases != 0 { fail = 6; return; }   // nothing released while handles live
}                                            // last release happens here

@main
func main() -> lang.i64 {
    exercise();
    if fail != 0 { return 1 }
    if releases != 1 { return 2 }            // captured resource released exactly once
    0
}
