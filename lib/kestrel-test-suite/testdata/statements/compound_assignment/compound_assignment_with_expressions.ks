// test: diagnostics
// stdlib: true

module Main

func getValue() -> Int { 10 }

func test() {
    var x: Int = 5;
    x += getValue();
    x *= 2 + 3;
}
