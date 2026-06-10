// test: diagnostics
// stdlib: false

// E212: a closure cannot capture a named ref binding — the environment
// would store the reference and the closure can outlive the borrow.
module Test

func f() {
    var x: lang.i64 = 1;
    let r = &x;
    let cl = { () in lang.i64_add(r, 1) }; // ERROR(E212)
}
