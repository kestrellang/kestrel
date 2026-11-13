// test: diagnostics
// stdlib: true

// Rebinding a slice `var` inside a branch would carry the new reference
// across the merge, which v1 rejects (E497).

module Test

import std.text.(String)

func pick(s: String, c: Bool) -> Int64 {
    var b = s.asSlice();
    if c { b = s.subslice(from: 0, to: 1) } // ERROR(E497)
    b.bytes.count
}
