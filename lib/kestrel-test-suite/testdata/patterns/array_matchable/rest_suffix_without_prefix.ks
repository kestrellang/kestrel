// test: diagnostics
// stdlib: true

module Test

import std.memory.Slice

func restAndLastTwo(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [..rest, y, z] => lang.i64_add(rest.count.raw, lang.i64_add(y, z)),
        _ => 0
    }
}
