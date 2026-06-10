// test: diagnostics
// stdlib: false

// Stage 0.5: references cannot be stored in fields (storable refs are the
// stage-2 default-don't-build question; nothing is reserved for them).
module Test

struct Holder {
    var r: &lang.i64 // ERROR(E483)
}
