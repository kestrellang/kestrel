// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 10;
    let y: std.num.Int64 = 32;
    if x + y != 42 { return 1 }
    0
}
