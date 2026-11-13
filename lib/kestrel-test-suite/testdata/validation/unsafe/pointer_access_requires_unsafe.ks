// test: diagnostics
// stdlib: true
//
// `Pointer`: creating, offsetting and casting are safe; touching the pointee
// (`read`, `write`, `value`, …) and building an `UnsafeSlice` are not.

module Test

func f() -> Int64 {
    var x: Int64 = 1;
    let p = Pointer(to: x);
    let q = p.offset(by: 0);
    let raw = q.asRaw();
    p.write(2); // ERROR: 'write' is unsafe and requires an `unsafe` expression or block
    let v = p.value; // ERROR: 'value' is unsafe and requires an `unsafe` expression or block
    let s = UnsafeSlice(pointer: p, count: 1); // ERROR: 'UnsafeSlice.init' is unsafe and requires an `unsafe` expression or block
    p.read() // ERROR: 'read' is unsafe and requires an `unsafe` expression or block
}

func g() -> Int64 {
    var x: Int64 = 1;
    let p = Pointer(to: x);
    unsafe p.write(2);
    let s = unsafe UnsafeSlice(pointer: p, count: 1);
    s.count + unsafe p.read()
}
