// test: execution
// stdlib: true
// expect-exit: 0

module Test

func double(n: Int64) -> Int64 { n * 2 }

func main() -> lang.i64 {
    let x = 3;
    let y = 4;

    if "sum: \(x + y)" != "sum: 7" { return 1 }

    if "call: \(double(5))" != "call: 10" { return 2 }

    if "expr: \(x * y + 1)" != "expr: 13" { return 3 }

    0
}
