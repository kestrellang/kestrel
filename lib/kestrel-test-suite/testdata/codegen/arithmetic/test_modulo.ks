// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let x: std.num.Int64 = 142;
    let y: std.num.Int64 = 100;
    if x % y != 42 { return 1 }
    0
}
