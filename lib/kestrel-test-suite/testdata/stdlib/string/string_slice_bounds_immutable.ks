// test: diagnostics
// stdlib: true

// A StringSlice's bounds live in its fileprivate `storage` (the pointer and
// length) — reaching it from outside the type must be rejected, otherwise
// rewriting it bypasses every bounds invariant the slice relies on.

module Test

func main() -> lang.i64 {
    let s: std.text.String = "hello";
    let slice = s.asSlice();
    let raw = slice.storage; // ERROR: member 'storage' is fileprivate and not accessible from this scope
    0
}
