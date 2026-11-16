// test: execution
// stdlib: true
// expect-exit: 0

// The right side of `and`/`or` is lowered into a thunk passed to
// `logicalAnd`/`logicalOr`. That thunk is a `mutating` closure: it runs at
// most once, right away, and is never stored, so it may write to the
// caller's variables. Assignments in an `if`/`match` arm and mutating method
// calls on the right side all write back, and a skipped right side writes
// nothing.
module Test

import std.numeric.Int64
import std.collections.Set

func bump(mutating n: Int64) -> Bool {
    n = n + 1;
    true
}

@main
func main() -> lang.i64 {
    // Assignment inside an `if` arm on the right side (E603 before).
    var x = 0;
    let a = true and (if x == 0 { x = 7; true } else { false });
    if not a { return 1; }
    if x != 7 { return 2; }

    // Assignment inside a `match` arm on the right side of `or`.
    let b = false or (match x { 7 => { x = 8; true }, _ => false });
    if not b { return 3; }
    if x != 8 { return 4; }

    // A skipped right side does not run, so it writes nothing.
    let c = false and (if true { x = 100; true } else { false });
    let d = true or (if true { x = 100; true } else { false });
    if c { return 5; }
    if not d { return 6; }
    if x != 8 { return 7; }

    // Passing a captured var to a `mutating` parameter writes back: four
    // iterations bump it (y = 5, 4, 3, 2); at y == 1 the right side is skipped.
    var count = 0;
    var y = 5;
    while y != 1 and bump(count) {
        y = y - 1;
    }
    if count != 4 { return 8; }

    // A mutating method call on a captured var writes back.
    var seen: Set[Int64] = [];
    var z = 3;
    while z != 0 and seen.insert(z) {
        z = z - 1;
    }
    if seen.count != 3 { return 9; }

    0
}
