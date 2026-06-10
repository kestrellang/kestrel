// test: execution
// stdlib: true
// backends: cranelift,llvm

// Stage 0.5 may-alias pin (the standing no-exclusivity decision): a
// `Pointer(to: x)` capture stays valid while `x` is simultaneously passed
// to a `mutating` param — both views alias the same place and the last
// write wins. There is no exclusivity check to violate.
module Test

import std.memory.(Pointer)
import std.numeric.(Int64)

// Writes through both views while both are live: the mutating param and
// the captured pointer alias the same storage.
func bumpThroughBoth(mutating n: Int64, p: Pointer[Int64]) {
    n = n + 1;        // through the mutating param: 5 -> 6
    p.write(p.read() + 10); // through the alias: 6 -> 16
}

@main
func main() -> lang.i64 {
    var x: Int64 = 5;
    let p = Pointer(to: x);
    bumpThroughBoth(x, p);
    // Both writes landed on the one place; the pointer write was last.
    if x != 16 { return 1; }

    // And the pointer still observes a direct write to `x` afterwards.
    x = 99;
    if p.read() != 99 { return 2; }
    0
}
