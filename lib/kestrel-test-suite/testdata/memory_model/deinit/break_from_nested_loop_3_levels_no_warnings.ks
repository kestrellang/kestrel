// test: diagnostics
// stdlib: true
//
// Paired sibling of `break_from_nested_loop_3_levels.ks`, which is an
// `execution` test — the harness only checks its exit code, so the analyzer
// warnings it provoked were invisible there. This file pins them: a
// `diagnostics` test fails on any unannotated warning, and there are no
// annotations here, so the shape must stay diagnostic-clean.
//
// G9: `break outer` from inside the nested loop exits the labeled outer loop,
// so `if deinit_count != 2 { ... }` after it is reachable. The dead-code walk
// used to stop at the nested `loop`, call the outer loop infinite, and emit a
// false E002 "unreachable code" on a line the execution test proves runs.

module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Resource: not Copyable {
    var id: Int64
    deinit {
        deinit_count = deinit_count + 1;
    }
}

func run() -> lang.i64 {
    let outer_r = Resource(id: 1);

    outer: loop {
        let mid_r = Resource(id: 2);

        loop {
            let inner_r = Resource(id: 3);
            break outer;
        }
    }

    if deinit_count != 2 { return 1; }
    0
}
