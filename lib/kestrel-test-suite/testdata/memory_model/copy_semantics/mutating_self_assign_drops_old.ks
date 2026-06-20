// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#141, facet 1): `self = new` in a mutating method of a
// non-Copyable type must DROP the old value before overwriting. A `mutating self`
// is a MutBorrow (inout) whose SSA value is `T` @guaranteed (the inout pointer),
// not `Pointer[T]` @owned, so the store-expansion's `MirTy::Pointer` drop-prefix
// did not fire and the old value leaked (deinit never ran). The fix normalizes
// the whole-self address to `Pointer[T]` so Take + __drop$T + StoreInit runs.

module Test

import std.numeric.Int64

public var drops: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    mutating func reset() { self = Res(id: 9); }
    deinit { drops = drops + 1; }
}

func consume(consuming r: Res) {}

@main
func main() -> lang.i64 {
    var v = Res(id: 1);
    v.reset();                 // old self (id 1) must drop here
    if drops != 1 { return 1 };
    consume(v);                // id 9 drops
    if drops != 2 { return 2 };
    0
}
