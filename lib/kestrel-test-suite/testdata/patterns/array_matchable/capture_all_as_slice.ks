// test: diagnostics
// stdlib: true

module Test

import std.memory.Slice

func asSliceLength(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [..all] => all.count.raw
    }
}
