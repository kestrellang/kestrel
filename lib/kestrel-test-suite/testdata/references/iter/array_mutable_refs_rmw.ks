// test: execution
// backends: cranelift,llvm
// stdlib: true

// Stage 2d stdlib surface: `Array.mutableRefs()` mutates elements in
// place — `x += v` RMW and `x = v` store-through both land in the
// array's buffer (verified through the value subscript).
module Test

@main
func main() -> lang.i64 {
    var b = [1, 2, 3];
    for x in b.mutableRefs() {
        x += 10;
    }
    if b(0) != 11 { return 1; }
    if b(1) != 12 { return 2; }
    if b(2) != 13 { return 3; }

    for x in b.mutableRefs() {
        x = 7;
    }
    if b(0) != 7 { return 4; }
    if b(2) != 7 { return 5; }
    0
}
