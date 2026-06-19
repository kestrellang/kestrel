// test: diagnostics
// stdlib: false

// Stage 2b: a ref FIELD is a legal formation (the struct is structurally
// non-Static; with the stdlib present, the 2a Static bound gates it out
// of heap/static/capture positions — see references/composition/). The
// old E483 rejection is carved out.
module Test

struct Holder {
    var r: &lang.i64
}
