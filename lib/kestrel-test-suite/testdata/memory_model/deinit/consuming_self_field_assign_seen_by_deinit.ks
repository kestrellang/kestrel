// test: execution
// stdlib: true
// expect-stdout: deinit live=0
//
// A field assigned on a `consuming self` must be the value `deinit` sees: the
// method owns `self`, so the write lands on the value that is dropped at the
// end of the call. MIR lowering used to write to a copy of `self`, so the
// original dropped with the old field (`live=1`); an assigned-through owned
// param now gets a stack slot. Found building `OwnedBox`, which clears a
// liveness flag this way in `consumePointee`.

module Test

import std.numeric.(Int64)
import std.io.stdio.(print)

struct Tok: not Copyable {
    var live: Int64
    consuming func finish() { self.live = 0; }
    deinit { print("deinit live=\(self.live)") }
}

@main func main() -> lang.i64 {
    let t = Tok(live: 1);
    t.finish();
    0
}
