// test: diagnostics
// stdlib: true

module Test

import std.memory.Slice

func sliceFirst(s: Slice[lang.i64]) -> lang.i64 {
    match s {
        [first, ..] => first,
        _ => 0
    }
}
