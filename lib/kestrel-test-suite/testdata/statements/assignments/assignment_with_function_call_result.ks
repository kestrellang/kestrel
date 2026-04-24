// test: diagnostics
// stdlib: false

module Main

func getValue() -> lang.i64 { 42 }

func test() -> lang.i64 {
    var x: lang.i64 = 0;
    x = getValue();
    x
}
