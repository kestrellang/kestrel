// test: diagnostics
// stdlib: false

// E-REF-13: a `consuming` receiver cannot root a returned reference —
// `self` is destroyed when the call returns, so the ref would dangle.
module Test

struct Box {
    var v: lang.i64

    consuming func take() -> &lang.i64 {
        self.v // ERROR(E496)
    }
}
