// test: execution
// stdlib: true

// Regression: a memberwise-aggregate (`Struct`/`Tuple`/`Enum`) whose field
// expressions split the current block (a `try`/`if`/`match` field value)
// double-dropped every NON-LAST droppable field.
//
// When a *later* field expression splits the block, the already-materialized
// values of the earlier fields are threaded through the new blocks and renamed
// at the merge (recorded in `value_forwarding`). The aggregate emitter held the
// *pre-split* SSA id, so it consumed the stranded id while the live threaded
// twin stayed tracked in scope — and was destroyed at scope exit. That spurious
// twin-drop ran the field's `deinit` a second time: an over-release that
// double-freed any COW-shared field (the kestrel-wall per-request String leak →
// use-after-free). Fix: `own_aggregate_element` resolves each element value
// through `value_forwarding` before consuming, mirroring `emit_call_inner`.
//
// Detector: `Resource.deinit` decrements a never-freed heap cell, so each
// Resource must be deinit'd exactly once. A spurious twin-drop decrements the
// cell an extra time; reading it back through the surviving struct field yields
// a value one-too-low. The LAST field is never threaded, so it is the control.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)
import std.core.(Bool)

struct Resource: not Copyable {
    var cell: Pointer[Int64]
    func value() -> Int64 { self.cell.read() }
    deinit { self.cell.write(self.cell.read() - 1); }
}

func mk(value v: Int64) -> Resource {
    let p = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    p.write(v);
    Resource(cell: p)
}

struct Trip: not Copyable {
    var a: Resource
    var b: Resource
    var c: Resource
}

// Each field value is an `if` expression: it splits the block while the
// aggregate is still being assembled, so fields `a` and `b` (non-last) are
// threaded across the merge of the *later* fields' `if`s.
func build(flag: Bool) -> Trip {
    Trip(
        a: if flag { mk(value: 10) } else { mk(value: 0) },
        b: if flag { mk(value: 20) } else { mk(value: 0) },
        c: if flag { mk(value: 30) } else { mk(value: 0) }
    )
}

func main() -> lang.i64 {
    let t = build(true);
    // Read each field before `t` itself drops. A spurious twin-drop would have
    // already run the field's deinit once, decrementing its cell.
    if t.a.value() != 10 { return 1; }   // non-last field (was double-dropped)
    if t.b.value() != 20 { return 2; }   // non-last field (was double-dropped)
    if t.c.value() != 30 { return 3; }   // last field — control, always correct
    0
}
