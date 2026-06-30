// test: execution
// stdlib: true
// backends: cranelift,llvm
//
// Regression: a whole-self store (`self = expr`) that is the FIRST
// initialization of `self` in an init body must lower to StoreInit, not
// StoreAssign. StoreAssign drops the OLD value at self's address first — but in
// an init self is uninitialized, so that drop frees garbage field pointers,
// corrupting the heap. The corruption only bites when the value is later read,
// so a constructed-but-unused result hid the bug.
//
// `String(from: CString)` is exactly this shape — its init does
// `self = String.fromBytesUnchecked(...)` for the non-empty case. Calling
// `fromBytesUnchecked` directly worked; going through the init SIGBUS'd on use.
// This surfaced as a SIGBUS in the notes-backend example whenever a TEXT column
// was read back (talon-sqlite builds `String(from: cstr)` per row).

module Test

import std.ffi.cstring.(CString)

@main
func main() -> lang.i64 {
    // round-trip a non-empty string through CString (the whole-self-store init)
    let original = "Lovelace";
    let cs = original.toCString();
    let back = String(from: cs);              // self = fromBytesUnchecked(...)
    if back != "Lovelace" { return 1; }       // USE the value (reads heap bytes)
    if back.byteCount != 8 { return 2; }

    // stress the heap: repeated round-trips must each yield a valid String
    var i: Int64 = 0;
    while i < 50 {
        let s = String(from: "iteration".toCString());
        if s != "iteration" { return 3; }
        i = i + 1;
    }
    0
}
