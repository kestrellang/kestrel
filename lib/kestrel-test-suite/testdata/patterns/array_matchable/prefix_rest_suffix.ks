// test: diagnostics
// stdlib: true

module Test

import std.memory.Slice

func middleLength(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [_, ..middle, _] => middle.count.raw,
        _ => 0
    }
}
