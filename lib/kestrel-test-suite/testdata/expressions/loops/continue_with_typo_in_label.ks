// test: diagnostics
// stdlib: false

module Main

func test() {
    var x: lang.i64 = 0;
    myloop: while lang.i64_signed_lt(x, 10) {
        x = lang.i64_add(x, 1);
        continue myloooop; // ERROR: undeclared label
    }
}
