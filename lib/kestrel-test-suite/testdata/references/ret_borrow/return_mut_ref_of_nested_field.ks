// test: execution
// stdlib: true
// backends: cranelift,llvm

// Field-address chains compose: `self.a.b` inherits Param(self) through
// BOTH FieldAddr projections, and the write through the returned ref
// lands in the nested storage.
module Test

struct Inner {
    var b: Int64
}

struct Outer {
    var a: Inner

    mutating func bRef() -> &mutating Int64 {
        self.a.b
    }
}

@main
func main() -> lang.i64 {
    var o = Outer(a: Inner(b: 3));
    o.bRef() = 30;
    if o.a.b != 30 { return 1; }
    0
}
