// test: execution
// stdlib: true
// backends: cranelift,llvm

// Arm-value decay (stage 1.5): a ref-returning call as a RAW arm value
// decays to an owned value at the merge — `match c { 1 => b.peek(), _ => 0 }`
// compiles and produces an owned result. Covers match, if, if-let, nested
// match, guarded arms, and single-arm match. The merge constraints stay
// Equal (bidirectional), so the literal arms still default normally.
module Test

import std.numeric.(Int64)

struct Box {
    var v: Int64
    func peek() -> &Int64 { self.v }
}

func maybe(c: Int64) -> Int64? {
    if c == 1 { .Some(c) } else { .None }
}

func mixedMatch(b: Box, c: Int64) -> Int64 {
    match c {
        1 => b.peek(),
        _ => 0,
    }
}

func mixedIf(b: Box, c: Int64) -> Int64 {
    if c == 1 { b.peek() } else { 0 }
}

func ifLetArm(b: Box, c: Int64) -> Int64 {
    if let .Some(n) = maybe(c) { b.peek() } else { 0 }
}

// Outer arm is itself a match: inner arms decay, outer mark is inert.
func nestedMatch(b: Box, c: Int64) -> Int64 {
    match c {
        1 => match c {
            1 => b.peek(),
            _ => 1,
        },
        _ => 0,
    }
}

// Guarded arm whose VALUE is a ref.
func guardedArm(b: Box, c: Int64) -> Int64 {
    match c {
        n if n == 1 => b.peek(),
        _ => 0,
    }
}

func singleArm(b: Box) -> Int64 {
    match 0 {
        _ => b.peek(),
    }
}

@main
func main() -> lang.i64 {
    let b = Box(v: 7);
    if mixedMatch(b, 1) != 7 { return 1; }
    if mixedMatch(b, 0) != 0 { return 2; }
    if mixedIf(b, 1) != 7 { return 3; }
    if mixedIf(b, 0) != 0 { return 4; }
    if ifLetArm(b, 1) != 7 { return 5; }
    if ifLetArm(b, 0) != 0 { return 6; }
    if nestedMatch(b, 1) != 7 { return 7; }
    if nestedMatch(b, 0) != 0 { return 8; }
    if guardedArm(b, 1) != 7 { return 9; }
    if guardedArm(b, 2) != 0 { return 10; }
    if singleArm(b) != 7 { return 11; }
    0
}
