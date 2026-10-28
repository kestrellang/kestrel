// test: execution
// stdlib: true
// expect-exit: 0

// Regression (fragility audit G1), failable-init half of the stopgap
// counterexample. See `init_field_reassign_default_copyable_field_drop.ks` for
// why this shape matters: `Res` has a `deinit` but no conformance clause, so it
// is DEFAULT-`Copyable` (`CopyBehavior::Bitwise`) and droppable at once, and
// `Wrap` — which only contains one — is `Bitwise` and droppable purely through
// `drop_fix::fix_drop_behaviors`. Neither the shipped `is_non_copyable`
// fallback nor a `copy_behavior != Bitwise` widening of it sees this field; the
// only thing that does is running `fix_drop_behaviors` BEFORE bodies are
// lowered, which `lower_items` now does.

module Test

import std.numeric.Int64

var dc: Int64 = 0;

struct Res {
    var id: Int64
    deinit { dc = dc + 1; }
}

struct Wrap {
    var r: Res
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
    // Failure return abandons a partially initialized `self`: `w` is live and
    // must be released.
    dc = 0;
    let failed = Container(value: -1);
    match failed {
        .Some(_) => { return 1 },
        _ => {}
    }
    if dc != 1 { return 2 }

    // Success path: the init drops nothing; `ok` leaving scope drops once.
    dc = 0;
    buildOk();
    if dc != 1 { return 3 }

    0
}
