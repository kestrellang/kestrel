// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 6;
    let y: std.num.Int64 = 7;
    if x * y != 42 { return 1 }
    0
}
