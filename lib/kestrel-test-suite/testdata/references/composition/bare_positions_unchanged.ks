// test: diagnostics
// stdlib: true

// References 2b: the bare-position rules are untouched by the aggregate
// carve — params, binding annotations, and fn-type returns still reject.
module Test

func takesBareRef(r: &Int64) -> Int64 { // ERROR(E480)
    0
}

func f() {
    var x = 1;
    let r: &Int64 = &x; // ERROR(E482)
    let g: () -> &Int64 = { 0 }; // ERROR(E486)
}
