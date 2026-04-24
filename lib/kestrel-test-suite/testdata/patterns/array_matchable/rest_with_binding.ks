// test: diagnostics
// stdlib: true

module Test

import std.memory.Slice

func restLength(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [_, ..rest] => rest.count.raw,
        _ => 0
    }
}
