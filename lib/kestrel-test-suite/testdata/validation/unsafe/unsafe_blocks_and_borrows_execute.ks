// test: execution
// stdlib: true
// expect-exit: 0
//
// `unsafe { … }` is a block whose last expression is its value, and
// `let r = unsafe &p.value;` binds a reference through a pointer.

module Test

@main
func main() -> Int64 {
    var x: Int64 = 41;
    let p = Pointer(to: x);
    let n = unsafe {
        p.write(p.read() + 1);
        p.read()
    };
    if n != 42 { return 1; }
    let r = unsafe &p.value;
    if r != 42 { return 2; }
    unsafe { p.write(7); }
    if unsafe p.read() != 7 { return 3; }
    0
}
