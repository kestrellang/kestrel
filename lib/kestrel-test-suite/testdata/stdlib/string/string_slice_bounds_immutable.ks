// test: diagnostics
// stdlib: true

// StringSlice.start/end are internal `let` fields — assigning them from outside
// the type must be rejected, otherwise `slice.start = -5` bypasses every
// bounds invariant the slice relies on.

module Test

func main() -> lang.i64 {
    let s: std.text.String = "hello";
    var slice = s.asSlice();
    slice.start = -5; // ERROR: member 'start' is internal and not accessible from this scope
    slice.end = 999; // ERROR: member 'end' is internal and not accessible from this scope
    0
}
