// test: diagnostics
// stdlib: true

module Test

import std.memory.ArraySlice

func sliceFirst(s: ArraySlice[lang.i64]) -> lang.i64 {
    match s {
        [first, ..] => first,
        _ => 0
    }
}
