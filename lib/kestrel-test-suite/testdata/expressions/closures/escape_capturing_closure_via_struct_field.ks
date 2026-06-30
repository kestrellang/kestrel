// test: diagnostics
// stdlib: true

module Test

import std.numeric.(Int64)

struct Holder {
    var f: () -> Int64
}

// Regression (#174): a capturing closure cannot escape even when laundered
// through a STRUCT FIELD. The struct construction joins the closure field's
// (frame-bound) root into the struct's root, and the deep escape gate
// (escape_carry recurses stored fields) runs the return check.
func make() -> Holder {
    let n: Int64 = 41;
    Holder(f: { () in n + 1 }) // ERROR(E494)
}

func main() -> lang.i64 { 0 }
