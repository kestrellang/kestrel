// test: execution
// stdlib: true
// expect-exit: 0

// Regression (fragility audit G1), and specifically the counterexample that
// rules out the cheap stopgap. The obvious narrow fix for G1 was to widen
// `setup_init_field_flags`'s `is_non_copyable` fallback to
// `copy_behavior != Bitwise`. That is INSUFFICIENT: a `deinit` does not affect
// copy semantics, so `Res` below — no conformance clause at all, hence
// DEFAULT-`Copyable` — is `CopyBehavior::Bitwise` *and* droppable, and `Wrap`,
// which merely contains one, is `Bitwise` too. Both fail `is_non_copyable`
// AND any `!= Bitwise` widening; `Wrap` is droppable only because
// `drop_fix::fix_drop_behaviors` walks its fields.
//
// The real fix is ordering: `lower_items` runs `fix_drop_behaviors` between the
// types pass and the functions pass, so `needs_drop` — the primary disjunct —
// already answers `true` by the time any init body is lowered.

module Test

import std.numeric.Int64

var dc: Int64 = 0;

// No conformance clause: default copy semantics (Bitwise). A `deinit` does not
// change that, so this type is copyable AND droppable at the same time.
struct Res {
    var id: Int64
    deinit { dc = dc + 1; }
}

// No `deinit`, no conformance clause: Bitwise, droppable only via `r`.
struct Wrap {
    var r: Res
}

struct Straight: not Copyable {
    var w: Wrap
    init() {
        self.w = Wrap(r: Res(id: 1));
        self.w = Wrap(r: Res(id: 2));   // must drop Wrap(Res(1))
    }
}

struct OneBranch: not Copyable {
    var w: Wrap
    init(flag c: Bool) {
        self.w = Wrap(r: Res(id: 10));
        if c { self.w = Wrap(r: Res(id: 11)); }   // drops Wrap(Res(10)) iff c
    }
}

func useStraight() { let x = Straight(); }
func useOneBranch(flag c: Bool) { let x = OneBranch(flag: c); }

@main
func main() -> lang.i64 {
    dc = 0; useStraight();             if dc != 2 { return 1 };
    dc = 0; useOneBranch(flag: true);  if dc != 2 { return 2 };
    dc = 0; useOneBranch(flag: false); if dc != 1 { return 3 };
    0
}
