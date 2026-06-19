// test: diagnostics
// stdlib: false

// Field addresses inherit their BASE's root — a field of a function-local
// `var` roots at the local's slot, so returning a ref to it is still the
// local-escape error (the provenance fix must not launder locals).
module Test

struct Box {
    var v: lang.i64
}

func bad() -> &mutating lang.i64 {
    var x = Box(v: 1);
    let r = &mutating x.v;
    r // ERROR(E494)
}
