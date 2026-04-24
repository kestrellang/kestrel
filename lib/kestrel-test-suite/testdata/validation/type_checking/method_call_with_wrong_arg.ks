// test: diagnostics
// stdlib: false

module Main

struct Calculator {
    var value: lang.i64

    func add(x: lang.i64) -> lang.i64 {
        lang.i64_add(self.value, x)
    }
}

func test() {
    let calc: Calculator = Calculator(value: 10);
    calc.add("five"); // ERROR
}
