// test: diagnostics
// stdlib: true

// References 2b: the bare-position rules are untouched by the aggregate
// carve — binding annotations and fn-type returns still reject. (A ref-typed
// param is a borrow, see references/params.)
module Test

func f() {
    var x = 1;
    let r: &Int64 = &x; // ERROR(E482)
    let g: () -> &Int64 = { 0 }; // ERROR(E486)
}
