// test: diagnostics
// stdlib: false

module Main

func test() -> () -> lang.i64 {
    var x = 10;
    {
        x = 20; // ERROR: cannot assign
        x
    }
}
