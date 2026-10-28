// test: diagnostics
// stdlib: false

// E-REF-11, the const-cast guard: `-> &mutating T` requires a MUTABLE
// root (a `mutating` receiver/param or `Pointer.mutatingValue`). A
// borrowing (shared) receiver cannot root a mutable reference.
module Test

struct Box {
    var v: lang.i64

    func bad() -> &mutating lang.i64 {
        self.v // ERROR(E495)
    }
}
