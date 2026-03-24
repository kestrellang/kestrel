// test: diagnostics
// stdlib: false

module Main

func test() {
    var x: lang.i64 = 0;
    sibling: while lang.i64_signed_lt(x, 10) {
        x = lang.i64_add(x, 1);
    }
    while lang.i64_signed_lt(x, 20) {
        x = lang.i64_add(x, 1);
        continue sibling; // ERROR: undeclared label
    }
}
