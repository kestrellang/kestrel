// test: diagnostics
// stdlib: true

module Test

import std.memory.Slice

func destructure(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [first, ..rest] => lang.i64_add(first, rest.count.raw),
        _ => 0
    }
}
