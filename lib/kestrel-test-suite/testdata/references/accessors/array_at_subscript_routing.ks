// test: execution
// stdlib: true
// backends: cranelift,llvm

// Array adoption gate (stage 1.5): the labeled `at:` subscript is the
// INHERENT in-place form (ref + mutating ref accessors); the unlabeled,
// `checked:` etc. subscripts keep routing to the Slice extension's
// get/set — distinct labels mean the label-based protocol-extension
// fallback needs no resolution changes. Read/write/RMW all pick the
// same provider set for the same spelling.
module Test

import std.numeric.(Int64)

@main
func main() -> lang.i64 {
    var arr = [11, 22, 33];

    // Inherent at: — in place.
    if arr(at: 0) != 11 { return 1; }
    arr(at: 1) = 220;
    if arr(at: 1) != 220 { return 2; }
    arr(at: 2) += 7;
    if arr(at: 2) != 40 { return 3; }

    // Unlabeled — Slice extension get/set, still routed via fallback.
    if arr(0) != 11 { return 4; }
    arr(0) = 110;
    if arr(at: 0) != 110 { return 5; }

    // Range subscript — the shape an inherent unlabeled Int64 subscript
    // would have captured and broken; `at:` doesn't.
    let mid = arr(1..<3);
    if mid.count != 2 { return 6; }
    if mid(0) != 220 { return 7; }

    // checked: — labeled extension form unaffected.
    match arr(checked: 99) {
        .Some(_) => { return 8; },
        .None => {},
    }

    // COW: writes through at: never touch a sibling copy.
    let snapshot = arr;
    arr(at: 0) = 9999;
    if snapshot(at: 0) != 110 { return 9; }
    if arr(at: 0) != 9999 { return 10; }
    0
}
