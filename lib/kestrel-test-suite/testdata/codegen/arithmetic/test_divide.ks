// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 84;
    let y: std.num.Int64 = 2;
    if x / y != 42 { return 1 }
    0
}
