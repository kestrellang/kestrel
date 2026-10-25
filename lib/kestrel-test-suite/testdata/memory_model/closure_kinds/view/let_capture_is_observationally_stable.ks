// test: execution
// stdlib: false
// expect-exit: 0

// docs/design/closures.md, "Normal: read-only views": a view over an IMMUTABLE
// binding is indistinguishable from a snapshot — nothing can write the place,
// so repeated calls agree. Pins that switching normal captures from snapshots
// to views is observationally inert for `let` captures.
module Test

@main
func main() -> lang.i64 {
    let k = 7;
    let f = { lang.i64_add(k, 1) };
    if lang.i64_eq(f(), 8) { } else { return 1; }
    if lang.i64_eq(f(), 8) { } else { return 2; }
    // copies of a normal closure share the frame's views and read the same value
    let g = f;
    if lang.i64_eq(g(), 8) { } else { return 3; }
    0
}
