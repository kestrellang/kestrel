// test: diagnostics
// stdlib: false
//
// #168: `some P` in struct-field position must be rejected with a clean
// diagnostic, not crash mir-lower ("opaque type origin has no concrete type").
// Opaque types are only valid in return position; a field has no
// return-position origin body to reify.

module Test

protocol Shape {
    func area() -> lang.i64
}

struct Circle {
    var r: lang.i64;
}
extend Circle: Shape {
    public func area() -> lang.i64 { self.r }
}

struct Holder {
    var s: some Shape; // ERROR: 'some' (opaque type) is not allowed in a field type
}
