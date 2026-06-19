// test: diagnostics
// stdlib: false

// Stage 1 carved the RETURN position out of the rejection walk for
// functions and computed-property getters — but NOT for subscripts: a
// DECLARED `-> &T` subscript stays E481 even now that stage 1.5 exists.
// In-place subscripts are spelled as `ref { … }` / `mutating ref { … }`
// accessor clauses (whose synthesized ref returns are carved separately);
// a ref type never appears in a subscript's source signature.
module Test

struct Box {
    var v: lang.i64

    subscript(i: lang.i64) -> &lang.i64 { // ERROR(E481)
        self.v
    }
}
