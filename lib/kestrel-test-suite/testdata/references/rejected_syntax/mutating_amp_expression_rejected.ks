// test: diagnostics
// stdlib: false

// `&mutating` in expression position (stage 1.5 item 2 surface): like bare
// `&`, a borrow expression is legal only as a `let` initializer — argument
// position stays E488 (borrowing is the signature's decision).
module Test

func takesBorrow(x: lang.i64) { }

func f() {
    var y: lang.i64 = 1;
    takesBorrow(&mutating y); // ERROR(E488)
}
