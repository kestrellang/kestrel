// test: diagnostics
// stdlib: false

// Stage 1 carved the RETURN position out of the rejection walk for
// functions and computed-property getters — but NOT for subscripts:
// get/set pairing plus `arr[i] = v` write-through needs the call-as-place
// grammar (stage 1.5), and the method spelling (`at(index:)`) loses
// nothing. A subscript ref return stays E481, so the wording's "yet"
// stays truthful.
module Test

struct Box {
    var v: lang.i64

    subscript(i: lang.i64) -> &lang.i64 { // ERROR(E481)
        self.v
    }
}
