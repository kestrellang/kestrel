// test: diagnostics
// stdlib: true

module Main

func test() -> lang.i64 {
    var num: lang.i64 = 0;
    num = 42;
    var text: lang.str = "";
    text = "assigned";
    var items: [lang.i64] = [];
    items = [1, 2];
    num
}
