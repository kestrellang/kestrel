// test: execution
// stdlib: true
// expect-exit: 0

// Positive contrast to escaping_param_truncated_to_normal_cannot_be_returned.ks
// (plan D5, conversion (2)): the rejection there is specific to the TRUNCATED
// VIEW. Returning a borrowed `escaping` parameter at `escaping` hands back a
// retained copy of an owned environment — self-rooted provenance — so E494
// must NOT fire, and the forwarded handle shares one environment with the
// caller's (reference semantics, docs/design/closures.md "escaping: a shared,
// stateful object").
module Test

import std.numeric.Int64

func take(f: escaping () -> Int64) -> escaping () -> Int64 {
    f                                    // retained copy, not a frame view
}

func makeCounter(start: Int64) -> escaping () -> Int64 {
    var count = start;
    { () in count = count + 1; count }
}

@main
func main() -> lang.i64 {
    let next = makeCounter(0);
    let forwarded = take(next);
    if forwarded() != 1 { return 1 }
    if next() != 2 { return 2 }          // one environment, two handles
    if forwarded() != 3 { return 3 }
    0
}
