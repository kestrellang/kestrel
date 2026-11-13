// test: diagnostics
// stdlib: true

// Only a `var` slice rebinds; assigning to a `let` one would write through
// a shared reference (E208).

module Test

import std.text.(String)

func pick(s: String) -> Int64 {
    let d = s.asSlice();
    d = s.trimmed(); // ERROR(E208)
    d.bytes.count
}
