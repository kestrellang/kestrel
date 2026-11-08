// test: execution
// stdlib: false
// backends: cranelift,llvm
//
// A local of an `@align(64)` type gets a stack slot aligned to 64 in both
// backends, including when the caller's frame is not.

module Main

@align(64)
struct Line { var a: lang.i64; var b: lang.i64 }

func misalignedOnPurpose(depth: lang.i64) -> lang.i64 {
    var pad: lang.i8 = 1;
    var line = Line(a: depth, b: 2);
    let p = lang.ptr_to(line);
    let addr = lang.ptr_to_address(p);
    if lang.i64_ne(lang.i64_and(addr, 63), 0) { return 1; }
    if lang.i64_ne(line.a, depth) { return 2; }
    if lang.i64_eq(depth, 0) { return 0; }
    misalignedOnPurpose(lang.i64_sub(depth, 1))
}

@main
func main() -> lang.i64 {
    misalignedOnPurpose(3)
}
