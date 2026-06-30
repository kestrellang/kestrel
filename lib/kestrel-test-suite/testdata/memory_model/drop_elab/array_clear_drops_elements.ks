// test: execution
// stdlib: true
// expect-exit: 0

// Regression (#155): `Array.clear()` must run the deinit of every live
// element. The old implementation set `storage.len = 0` directly, so
// `ArrayStorage.deinit` (which only drops `0..<len`) saw an empty array and
// skipped every destructor — leaking all elements. Each `Res.deinit` bumps a
// global counter; clearing a 2-element array must drop exactly 2 (checking
// `== 2` also guards against an over-eager double-drop). Before the fix the
// count was 0 (both elements leaked, never dropped even at scope exit).

module Test

import std.numeric.Int64

public var drops: Int64 = 0;

struct Res: Copyable {
    var id: Int64
    deinit { drops = drops + 1; }
}

@main
func main() -> lang.i64 {
    var arr = Array[Res]();
    arr.append(Res(id: 1));
    arr.append(Res(id: 2));
    arr.clear();
    if drops != 2 { return 1 }   // clear() must deinit both elements, exactly once each
    0
}
