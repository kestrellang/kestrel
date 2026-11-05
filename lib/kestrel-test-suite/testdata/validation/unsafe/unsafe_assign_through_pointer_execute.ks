// test: execution
// stdlib: true
// expect-exit: 0
//
// `unsafe p.value op= v` and `unsafe p.value = v`: the prefix covers the
// place, and the assignment still sees it as a place.

module Test

@main
func main() -> Int64 {
    var x: Int64 = 40;
    let p = Pointer(to: x);
    unsafe p.mutatingValue += 2;
    if unsafe p.read() != 42 { return 1; }
    unsafe p.mutatingValue = 7;
    if unsafe p.read() != 7 { return 2; }
    0
}
