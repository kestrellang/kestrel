// test: diagnostics
// stdlib: true

module Test

import std.memory.Slice

func destructure(arr: [lang.i64]) -> lang.i64 {
    let [..all] = arr;
    all.count.raw
}
