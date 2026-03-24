// test: execution
// stdlib: true

module Test

func make_doubler() -> (std.num.Int64) -> std.num.Int64 {
    { (x) in x * 2 }
}

func main() -> lang.i64 {
    let doubler = make_doubler();
    if doubler(21) != 42 { return 1 }
    0
}
