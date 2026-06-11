// test: execution
// stdlib: true
// backends: cranelift,llvm

// The provenance fix isn't `&mutating`-specific: a SHARED `&T` return of
// a field projection through an ADDRESSED (mutating) receiver takes the
// same field-address route and must root at Param(self), not the temp.
module Test

struct Box {
    var v: Int64

    mutating func view() -> &Int64 {
        self.v
    }
}

@main
func main() -> lang.i64 {
    var b = Box(v: 41);
    let got = b.view();
    if got != 41 { return 1; }
    b.v = 8;
    if b.view() != 8 { return 2; }
    0
}
