// test: diagnostics
// stdlib: true

module Test

import std.memory.Slice

func complexPattern(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [a, b, ..middle, y, z] => lang.i64_add(lang.i64_add(a, b), lang.i64_add(y, z)),
        _ => 0
    }
}
