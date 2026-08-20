// test: execution
// stdlib: true
// expect-exit: 0

// Regression (fragility audit G1): reassigning an already-initialized `self`
// field inside an init body must drop the old value even when the field's type
// is droppable ONLY TRANSITIVELY — it has no `deinit` of its own, just a
// droppable field — and is `Cloneable`, so the `is_non_copyable` fallback in
// `setup_init_field_flags` does not cover it either.
//
// `lower_drop_behavior` reports only a *user* `deinit`, so `Wrap` came out of
// the types pass as `DropBehavior::None`. The pass that promotes it,
// `drop_fix::fix_drop_behaviors`, had exactly one call site — the
// `Stage::DropFix` slot in `passes::run_pipeline_until` — which runs AFTER
// every function body has been lowered. `setup_init_field_flags` therefore saw
// a non-droppable field, allocated no drop flag, and emitted a plain
// `store_init` for the second assignment instead of the `store_assign` that
// carries the destroy-old expansion. The abandoned value was never released
// (measured with a real `String` field: 400 leaks / 265600 bytes over 200
// iterations, against 0 for the single-assignment control). `lower_items` now
// runs `fix_drop_behaviors` between the types pass and the functions pass.
//
// Note the shape: every OTHER fixture in this directory declares
// `struct …: not Copyable` *and* gives it a `deinit`, which satisfies BOTH
// disjuncts of the droppability test — the suite was structurally blind to a
// wrapper that is droppable but copyable.

module Test

import std.numeric.Int64

var dc: Int64 = 0;

// Has its own `deinit` → already droppable when the types pass finishes.
struct Res: Cloneable {
    var id: Int64
    func clone() -> Res { Res(id: self.id) }
    deinit { dc = dc + 1; }
}

// NO `deinit`. Droppable purely because it contains a `Res` — a fact that only
// `fix_drop_behaviors` establishes. `Cloneable`, so `is_non_copyable` is false.
struct Wrap: Cloneable {
    var r: Res
    func clone() -> Wrap { Wrap(r: self.r.clone()) }
}

// straight-line reassignment through the transitively-droppable wrapper.
struct Straight: not Copyable {
    var w: Wrap
    init() {
        self.w = Wrap(r: Res(id: 1));
        self.w = Wrap(r: Res(id: 2));   // must drop Wrap(Res(1))
    }
}

// each branch assigns once, field uninit on entry → NO drop inside the init.
struct Cond: not Copyable {
    var w: Wrap
    init(flag c: Bool) {
        if c { self.w = Wrap(r: Res(id: 1)); } else { self.w = Wrap(r: Res(id: 2)); }
    }
}

// definitely-init, then reassign on one branch only.
struct OneBranch: not Copyable {
    var w: Wrap
    init(flag c: Bool) {
        self.w = Wrap(r: Res(id: 10));
        if c { self.w = Wrap(r: Res(id: 11)); }   // drops Wrap(Res(10)) iff c
    }
}

func useStraight() { let x = Straight(); }
func useCond(flag c: Bool) { let x = Cond(flag: c); }
func useOneBranch(flag c: Bool) { let x = OneBranch(flag: c); }

@main
func main() -> lang.i64 {
    // Both the reassigned-away value AND the final one must be released.
    dc = 0; useStraight();             if dc != 2 { return 1 };
    dc = 0; useCond(flag: true);       if dc != 1 { return 2 };
    dc = 0; useCond(flag: false);      if dc != 1 { return 3 };
    dc = 0; useOneBranch(flag: true);  if dc != 2 { return 4 };
    dc = 0; useOneBranch(flag: false); if dc != 1 { return 5 };
    0
}
