// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#180): `for-in` over Array[Cloneable] must clone each element AT
// MOST ONCE per iteration (the iterator yield), not twice. Before the fix the
// loop-pattern binding clone-copied the already-@owned yielded element a second
// time (borrow-extract-copy in emit_enum_payload), so each element was cloned
// twice and its deinit ran twice in-loop. The fix routes @owned non-bitwise-
// Copyable payload extraction through emit_moveout (move, no clone). Both
// backends exhibited the double-clone.
//
// Two elements, empty loop body: exactly 2 clones (one per element) and exactly
// 2 in-loop deinits (each clone dropped at end of its iteration). The two
// originals remain in the array and drop only when `items` drops at scope exit
// (not asserted). Before the fix: 4 clones / 4 in-loop deinits.

module Test

import std.numeric.Int64

public var clones: Int64 = 0;
public var deinits: Int64 = 0;

struct Item: Cloneable {
    var id: Int64
    func clone() -> Item { clones = clones + 1; return Item(id: self.id); }
    deinit { deinits = deinits + 1; }
}

@main
func main() -> lang.i64 {
    var items: Array[Item] = [];
    items.append(Item(id: 1));
    items.append(Item(id: 2));
    for item in items {
    }
    if clones != 2 { return 1 };     // one clone per element (was 4)
    if deinits != 2 { return 2 };    // each clone dropped in-loop; originals still live (was 4)
    0
}
