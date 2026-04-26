// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let a: std.num.Int64 = 10;
    let b: std.num.Int64 = 20;
    let c: std.num.Int64 = 12;
    let f = { a + b + c };
    if f() != 42 { return 1 }
    0
}
