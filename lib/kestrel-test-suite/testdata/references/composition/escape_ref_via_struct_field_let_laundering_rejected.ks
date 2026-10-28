// test: diagnostics
// stdlib: true

// Sibling of the closure-escape hole (#174): a reference laundered through a
// `let` binding into a user STRUCT FIELD and returned must be rejected. The
// deep escape gate (escape_carry recurses stored fields, not just type args)
// now sees the `&Int64` field on the non-generic `Holder`, and the struct
// construction roots the value at the borrowed local.
module Test

import std.numeric.(Int64)

struct Holder {
    var r: &Int64
}

func make() -> Holder {
    let n: Int64 = 41;
    let rr = &n;
    Holder(r: rr) // ERROR(E494)
}

func main() -> lang.i64 { 0 }
