// test: diagnostics
// stdlib: false

module Main

func test() {
    var localX: lang.i64 = 0;
    if true {
        localX = 10;
    }
}
