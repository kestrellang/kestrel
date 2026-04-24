// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let captured: std.num.Int64 = 32;
    let f = { (x: std.num.Int64) in x + captured };
    if f(10) != 42 { return 1 }
    0
}
