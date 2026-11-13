// test: diagnostics
// stdlib: true

// `and` binds tighter than a `let` condition, so `if let .Some(e) = end and
// start < e` would match against `end and start < e`, with `e` not yet bound.
// E160 points at the `,` condition list instead of the old cascade
// ("Optional[Int64] !: And", "undefined name 'e'"). A parenthesized `and` is
// an ordinary Bool scrutinee and stays legal.
module Test

import std.numeric.(Int64)

func check(end: Int64?, start: Int64) {
    if let .Some(e) = end and start < e {} // ERROR(E160)
    while let .Some(g) = end and start < g { break } // ERROR(E160)
    guard let .Some(f) = end and start < f else { return }; // ERROR(E160)
    if let .Some(e) = end, start < e {}
    let a = true;
    if let true = (a and start > 0) {}
}
