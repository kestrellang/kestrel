// test: diagnostics
// stdlib: true
//
// Regression (#163): a non-Copyable value consumed inside a `while` body
// without reassignment is (maybe-)moved on the next iteration's back edge —
// even with no use after the loop. Before the fix the move checker only
// promoted moves to the post-loop state and never flagged the back-edge re-use,
// so the invalid program reached MIR and ICE'd with a loop block-arg arity
// mismatch ("terminator passes N args ... but block expects N+1 params").

module Test

import std.numeric.Int64

struct Res: not Copyable {
    var id: Int64
    deinit {}
}

func consume(consuming r: Res) {}

@main
func main() -> lang.i64 {
    let r = Res(id: 1);
    var i: Int64 = 0;
    while i < 3 {
        consume(r); // ERROR: value 'r' may have been moved
        i = i + 1;
    }
    return 0;
}
