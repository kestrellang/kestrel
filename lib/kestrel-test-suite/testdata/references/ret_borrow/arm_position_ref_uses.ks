// test: execution
// stdlib: true
// backends: cranelift,llvm

// Refs used INSIDE `if`/`match` arms and loop conditions end cleanly at
// the arm tail — no false E497 (ref-across-merge) and no leaked borrow.
// Pins the arm-position watch-item from the stage-1 sign-off: every use
// here is a place context (operator receiver, binding decay, borrow-conv
// argument, loop condition) whose ref dies before the jump to the merge.
// NOTE: a ref as the raw arm VALUE (`1 => b.peek(),`) is still a type
// error — arm merges unify with Equal, not Coerce (stage-1.5 follow-up).
module Test

import std.numeric.(Int64)

struct Box {
    var v: Int64
    func peek() -> &Int64 { self.v }
}

func took(n: Int64) -> Int64 { n }

// Operator through a ref as a match-arm value: owned result, ref ends
// before the jump to merge.
func matchArm(b: Box, c: Int64) -> Int64 {
    match c {
        1 => b.peek() + 1,
        _ => 0,
    }
}

// Same inside an if-expression arm.
func ifArm(b: Box, c: Int64) -> Int64 {
    if c == 1 { b.peek() + 1 } else { 0 }
}

// Ref born + bound (binding decay) inside a match-arm block.
func armBlock(b: Box, c: Int64) -> Int64 {
    match c {
        1 => {
            let x = b.peek();
            x + 1
        },
        _ => 0,
    }
}

// While condition reads through a ref on every iteration (back-edge).
func loopCondition(b: Box) -> Int64 {
    var n: Int64 = 0;
    while n < b.peek() {
        n = n + 1;
    }
    n
}

// Ref as a borrow-convention call argument inside an arm.
func argInArm(b: Box, c: Int64) -> Int64 {
    match c {
        1 => took(b.peek()),
        _ => 0,
    }
}

@main
func main() -> lang.i64 {
    let b = Box(v: 2);
    if matchArm(b, 1) != 3 { return 1; }
    if matchArm(b, 0) != 0 { return 2; }
    if ifArm(b, 1) != 3 { return 3; }
    if armBlock(b, 1) != 3 { return 4; }
    if loopCondition(b) != 2 { return 5; }
    if argInArm(b, 1) != 2 { return 6; }
    0
}
