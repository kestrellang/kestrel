// test: execution
// stdlib: true

// Regression (kestrel-wall trigger): a memberwise-aggregate whose field values
// are `try` expressions double-dropped every NON-LAST field. The `try` splits
// the current block into success/error-propagation edges; the earlier fields'
// values are threaded across the merge and renamed (`value_forwarding`), but the
// aggregate emitter consumed the stranded pre-split id while the live threaded
// twin was destroyed at scope exit — a spurious second `deinit`. With wall's
// COW-shared `WallNote` String columns that over-release was a use-after-free.
// Fix: `own_aggregate_element` resolves each element through `value_forwarding`.
//
// Companion to `aggregate_control_flow_field_no_double_drop.ks` (the `if` form);
// this one pins the `try` block-split path that wall actually hit.
//
// Detector: `Resource.deinit` decrements a never-freed heap cell; a spurious
// twin-drop reads back one-too-low through the surviving struct field.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

enum E { case Boom }

struct Resource: not Copyable {
    var cell: Pointer[Int64]
    func value() -> Int64 { self.cell.read() }
    deinit { self.cell.write(self.cell.read() - 1); }
}

func mk(value v: Int64) -> Result[Resource, E] {
    let p = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    p.write(v);
    .Ok(Resource(cell: p))
}

struct Trip: not Copyable {
    var a: Resource
    var b: Resource
    var c: Resource
}

// `try` field values split the block during aggregate assembly. Fields `a` and
// `b` (non-last) are threaded across the merges of the later fields' `try`s.
func build() -> Result[Trip, E] {
    .Ok(Trip(
        a: try mk(value: 10),
        b: try mk(value: 20),
        c: try mk(value: 30)
    ))
}

@main
func main() -> lang.i64 {
    match build() {
        .Ok(t) => {
            // Read each field before `t` drops. A spurious twin-drop already ran
            // the field's deinit once, decrementing its cell.
            if t.a.value() != 10 { return 1; }   // non-last field
            if t.b.value() != 20 { return 2; }   // non-last field
            if t.c.value() != 30 { return 3; }   // last field — control
            0
        },
        .Err(_) => 4
    }
}
