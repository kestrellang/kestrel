// test: execution
// stdlib: true
// expect-exit: 0

// Regression (fragility audit G1), failable-init half. Sibling of
// `partial_drop_on_init_failure.ks`, but the already-assigned field's type is
// droppable ONLY TRANSITIVELY (no `deinit` of its own) and is `Cloneable`.
//
// `setup_init_field_flags` decides which `self` fields get a drop flag by
// asking `needs_drop`, which reads `type_info.drop`. Before the fix that flag
// was still `DropBehavior::None` for `Wrap` at body-lowering time — the pass
// that promotes it (`drop_fix::fix_drop_behaviors`) ran later, from
// `passes::run_pipeline_until`. With no flag, the failure return emitted none
// of the guarded-destroy diamond (`field_addr` / `load` the flag / `branch` to
// `destroy_addr`) that the `deinit`-bearing control gets, so the already-built
// `Wrap` was abandoned. `lower_items` now runs `fix_drop_behaviors` between the
// types pass and the functions pass.

module Test

import std.numeric.Int64

var dc: Int64 = 0;

struct Res: Cloneable {
    var id: Int64
    func clone() -> Res { Res(id: self.id) }
    deinit { dc = dc + 1; }
}

// No `deinit`; droppable only via its `Res` field. `Cloneable`, so the
// `is_non_copyable` fallback does not rescue it either.
struct Wrap: Cloneable {
    var r: Res
    func clone() -> Wrap { Wrap(r: self.r.clone()) }
}

struct Container: not Copyable {
    var w: Wrap
    var value: Int64

    init(value value: Int64)? {
        self.w = Wrap(r: Res(id: 7));
        if value < 0 { return null }
        self.value = value;
    }
}

func buildOk() {
    let ok = Container(value: 42);
    match ok {
        .Some(c) => {},
        _ => {}
    }
}

@main
func main() -> lang.i64 {
    // Failure return: `w` is initialized, `value` is not. The partially
    // initialized `self` must release `w` — and therefore its `Res`.
    dc = 0;
    let failed = Container(value: -1);
    match failed {
        .Some(_) => { return 1 },
        _ => {}
    }
    if dc != 1 { return 2 }

    // Success path: nothing is dropped on the way out of `init` (the caller
    // owns the object); the single drop comes from `ok` leaving scope.
    dc = 0;
    buildOk();
    if dc != 1 { return 3 }

    0
}
