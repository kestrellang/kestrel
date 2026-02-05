module Test

func main() -> lang.i64 {
    let a: std.num.Int64 = 10;
    let b: std.num.Int64 = 20;

    // This should resolve notEquals from Equatable extension
    // The parameter should be Int64, not Self
    let result = a.notEquals(b);

    if result { 0 } else { 1 }
}
