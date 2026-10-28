// test: execution
// stdlib: true
// backends: cranelift,llvm

// Arm-value decay (stage 1.5): even when EVERY arm is a ref, the merge
// result is OWNED — refs cannot cross merges, so each arm copies out and
// ends its borrow before the jump (no E497). The post-merge mutation
// probes prove the result does not alias the source.
module Test

import std.numeric.(Int64)

struct Box {
    var v: Int64
    var w: Int64
    func peekV() -> &Int64 { self.v }
    func peekW() -> &Int64 { self.w }
}

func pickMatch(b: Box, c: Int64) -> Int64 {
    match c {
        1 => b.peekV(),
        _ => b.peekW(),
    }
}

func pickIf(b: Box, c: Int64) -> Int64 {
    if c == 1 { b.peekV() } else { b.peekW() }
}

@main
func main() -> lang.i64 {
    var b = Box(v: 10, w: 20);
    if pickMatch(b, 1) != 10 { return 1; }
    if pickMatch(b, 0) != 20 { return 2; }

    // Owned, not aliased: mutating the source after the merge leaves the
    // already-produced results untouched.
    let x = pickIf(b, 1);
    let y = pickIf(b, 0);
    b.v = 99;
    b.w = 99;
    if x != 10 { return 3; }
    if y != 20 { return 4; }
    if pickIf(b, 1) != 99 { return 5; }
    0
}
