// test: execution
// stdlib: true
// expect-exit: 0

// #107: a `let` conditionally moved before an early `return` must drop
// exactly once — the terminating-exit walker emits a flag-guarded destroy
// for the MaybeUninit slot instead of an unconditional DestroyAddr
// (double-free on the moved path) or an eager merge-point drop.
// Sibling coverage: conditional_move_var_exit_drops_once.ks (var flavor),
// conditional_move_let_arm_local_scope_exit.ks (fallthrough arm exit).

module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    deinit {
        deinit_count = deinit_count + 1;
    }
}

func consume(consuming r: Res) {}

func earlyReturn(cond: Bool) -> Int64 {
    let a = Res(id: 1);
    if cond {
        consume(a);
    } else {
    }
    if deinit_count > 100 { return 99; }
    return 7;
}

@main
func main() -> lang.i64 {
    let _r1 = earlyReturn(true);
    if deinit_count != 1 { return 1; }
    let _r2 = earlyReturn(false);
    if deinit_count != 2 { return 2; }
    0
}
