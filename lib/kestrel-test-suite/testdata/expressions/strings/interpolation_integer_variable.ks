// test: execution
// stdlib: true
// expect-exit: 0

module Test

@main
func main() -> lang.i64 {
    let x = 42;
    let result = "value is \(x)";
    if result != "value is 42" { return 1 }

    let neg = -7;
    if "neg: \(neg)" != "neg: -7" { return 2 }

    if "\(0)" != "0" { return 3 }

    0
}
