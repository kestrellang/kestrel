// test: diagnostics
// stdlib: true

module Test

import std.numeric.(Int64)

struct Holder {
    var f: () -> Int64 // ERROR(E626)
}

// Regression (#174): a capturing closure cannot escape when laundered
// through a STRUCT FIELD. Closure fields must be owning (E626,
// docs/design/closure-fields.md), so the view-closure field is rejected at
// its declaration and the escape cannot be written.
func make() -> Holder {
    let n: Int64 = 41;
    Holder(f: { () in n + 1 })
}

func main() -> lang.i64 { 0 }
