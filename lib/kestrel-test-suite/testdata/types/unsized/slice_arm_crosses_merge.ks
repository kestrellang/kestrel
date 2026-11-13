// test: diagnostics
// stdlib: true

// An `if`/`match` arm whose value is a slice has nothing to copy out at the
// merge: the arm would carry a reference across it, which v1 rejects (E497)
// instead of reporting a move out of a borrow.

module Test

import std.text.(String)

func pick(s: String, c: Bool) -> Int64 {
    let a = if c { s.trimmed() } else { s.trimmedStart() }; // ERROR(E497)
    a.bytes.count
}
