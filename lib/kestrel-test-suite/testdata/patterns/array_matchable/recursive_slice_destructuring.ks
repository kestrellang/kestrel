// test: diagnostics
// stdlib: true

module Test

import std.memory.Slice

func nestedMatch(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [first, ..rest] => {
            match rest {
                [second, ..] => lang.i64_add(first, second),
                _ => first
            }
        },
        _ => 0
    }
}
