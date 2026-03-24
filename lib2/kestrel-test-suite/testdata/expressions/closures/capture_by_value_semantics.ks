// test: diagnostics
// stdlib: false

module Main

func test() -> () -> lang.i64 {
    var x = 10;
    let f = { x };
    x = 20;
    f
}
