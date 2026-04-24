// test: execution
// stdlib: true

module Test

struct Calculator {
    let compute: (std.num.Int64, std.num.Int64) -> std.num.Int64
}

func run_calc(calc: Calculator, a: std.num.Int64, b: std.num.Int64) -> std.num.Int64 {
    (calc.compute)(a, b)
}

func main() -> lang.i64 {
    let adder = Calculator(compute: { (x, y) in x + y });
    if run_calc(adder, 20, 22) != 42 { return 1 }
    0
}
