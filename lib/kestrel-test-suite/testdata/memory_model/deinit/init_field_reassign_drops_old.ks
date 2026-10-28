// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#154): reassigning an already-initialized non-Copyable `self`
// field inside an init body must drop the old value. Init-body field stores
// used unconditional `store_init` (never dropping), keyed only on a binary
// is-init-self flag with no per-field initialized-state tracking — the leak
// was admitted in a code comment. Fields now carry the `VarInit` definite-
// initialization lattice (mirroring `var` slots): DefUninit → store_init;
// DefInit → store_assign (drops old); MaybeUninit → flag-guarded drop. Covers
// straight-line reassign, conditional init (no spurious drop), one-branch
// reassign, a MaybeUninit-then-reassign, and loop reassignment, on both backends.

module Test

import std.numeric.Int64

var dc: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    deinit { dc = dc + 1; }
}

// straight-line reassignment — the canonical #154 case.
struct Straight: not Copyable {
    var a: Res
    init() {
        self.a = Res(id: 1);
        self.a = Res(id: 2);   // must drop Res(1)
    }
}

// conditional init: each branch assigns once, field uninit on entry → NO drop.
struct Cond: not Copyable {
    var a: Res
    init(flag c: Bool) {
        if c { self.a = Res(id: 1); } else { self.a = Res(id: 2); }
    }
}

// definitely-init then reassign on one branch only.
struct OneBranch: not Copyable {
    var a: Res
    init(flag c: Bool) {
        self.a = Res(id: 10);
        if c { self.a = Res(id: 11); }   // drops Res(10) only when c
    }
}

// maybe-init (assigned on one branch) then unconditional reassign: the second
// store is MaybeUninit → flag-guarded drop (drops only if the branch ran).
struct MaybeThenReassign: not Copyable {
    var a: Res
    init(flag c: Bool) {
        if c { self.a = Res(id: 20); }   // a: MaybeUninit after the if
        self.a = Res(id: 21);            // flag-guarded: drops Res(20) iff c
    }
}

// plain (non-failable) init with an explicit early `return ()`: must NOT be
// misclassified as a failable-init failure exit and partial-drop the already-
// assigned field (the field-flag generalization made `init_field_flags`
// non-empty for plain inits, so the failure gate now also checks
// `is_failable_init` — without it this double-freed `a`).
struct PlainEarlyReturn: not Copyable {
    var a: Res
    init(flag c: Bool) {
        self.a = Res(id: 1);
        if c { return (); }
    }
}

// loop reassignment: each iteration drops the previous value.
struct LoopReassign: not Copyable {
    var a: Res
    init() {
        self.a = Res(id: 100);
        var i = 0;
        while i < 3 { self.a = Res(id: i); i = i + 1; }   // drops Res(100), Res(0), Res(1)
    }
}

func useStraight() { let x = Straight(); let _ = x.a.id; }
func useCond(flag c: Bool) { let x = Cond(flag: c); let _ = x.a.id; }
func useOneBranch(flag c: Bool) { let x = OneBranch(flag: c); let _ = x.a.id; }
func useMaybe(flag c: Bool) { let x = MaybeThenReassign(flag: c); let _ = x.a.id; }
func useEarly(flag c: Bool) { let x = PlainEarlyReturn(flag: c); let _ = x.a.id; }
func useLoop() { let x = LoopReassign(); let _ = x.a.id; }

@main
func main() -> lang.i32 {
    dc = 0; useStraight();          if dc != 2 { return 1 };  // drop Res(1) + Res(2) at exit
    dc = 0; useCond(flag: true);    if dc != 1 { return 2 };  // just the scope-exit drop
    dc = 0; useCond(flag: false);   if dc != 1 { return 3 };
    dc = 0; useOneBranch(flag: true);  if dc != 2 { return 4 };  // drop Res(10) + Res(11) exit
    dc = 0; useOneBranch(flag: false); if dc != 1 { return 5 };  // no reassign, Res(10) exit
    dc = 0; useMaybe(flag: true);   if dc != 2 { return 6 };  // drop Res(20) + Res(21) exit
    dc = 0; useMaybe(flag: false);  if dc != 1 { return 7 };  // no Res(20), Res(21) exit only
    dc = 0; useEarly(flag: true);   if dc != 1 { return 8 };  // early return must NOT partial-drop
    dc = 0; useEarly(flag: false);  if dc != 1 { return 9 };
    dc = 0; useLoop();              if dc != 4 { return 10 }; // 3 loop drops + 1 exit
    return 0;
}
