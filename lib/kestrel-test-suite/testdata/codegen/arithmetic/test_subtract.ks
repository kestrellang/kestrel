// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 50;
    let y: std.num.Int64 = 8;
    if x - y != 42 { return 1 }
    0
}
