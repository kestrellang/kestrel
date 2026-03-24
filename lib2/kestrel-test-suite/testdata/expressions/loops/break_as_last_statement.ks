// test: diagnostics
// stdlib: false

module Main

func test() {
    var x: lang.i64 = 0;
    while true {
        x = lang.i64_add(x, 1);
        break
    }
}
