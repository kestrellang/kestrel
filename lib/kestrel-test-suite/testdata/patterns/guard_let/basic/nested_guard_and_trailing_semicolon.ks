// test: execution
// stdlib: true
// expect-exit: 0

// A guard may nest inside another guard's else block, and a guard may end
// with a `;` like a `let`. Both used to be parse errors.

module Test

import std.numeric.Int64
func f(a: Int64?, b: Int64?) -> Int64 {
    guard let .Some(x) = a else {
        guard let .Some(y) = b else { return 0 }
        return y + 100
    }
    guard let .Some(z) = b else { return x };
    x + z
}
@main func main() -> lang.i32 {
    if f(.None, .None) != 0 { return 10 }
    if f(.None, .Some(2)) != 102 { return 11 }
    if f(.Some(3), .None) != 3 { return 12 }
    if f(.Some(3), .Some(4)) != 7 { return 13 }
    0
}
