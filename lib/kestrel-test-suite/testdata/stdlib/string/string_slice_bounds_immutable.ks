// test: diagnostics
// stdlib: true

// StringSlice.start/end are `let` fields — assigning them from outside
// the type must be rejected, otherwise `slice.start = -5` bypasses every
// bounds invariant the slice relies on.

module Test

func main() -> lang.i64 {
    let s: std.text.String = "hello";
    var slice = s.asSlice();
    slice.start = -5; // ERROR: cannot assign
    slice.end = 999; // ERROR: cannot assign
    0
}
