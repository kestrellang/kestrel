// test: execution
// backends: cranelift,llvm
// stdlib: true

// Stage 2d stdlib surface: `mutableRefs()` runs the COW barrier BEFORE
// handing out the buffer — writes through the yielded references never
// leak into a shared copy.
module Test

@main
func main() -> lang.i64 {
    var c = [5, 6];
    let snapshot = c;
    for x in c.mutableRefs() {
        x = 0;
    }
    if c(0) != 0 { return 1; }
    if c(1) != 0 { return 2; }
    if snapshot(0) != 5 { return 3; }
    if snapshot(1) != 6 { return 4; }
    0
}
