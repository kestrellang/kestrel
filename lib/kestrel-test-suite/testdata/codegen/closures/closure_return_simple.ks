// test: execution
// stdlib: true

module Test

func make_doubler() -> (std.numeric.Int64) -> std.numeric.Int64 {
    { (x) in x * 2 }
}

@main
func main() -> lang.i64 {
    let doubler = make_doubler();
    if doubler(21) != 42 { return 1 }
    0
}
