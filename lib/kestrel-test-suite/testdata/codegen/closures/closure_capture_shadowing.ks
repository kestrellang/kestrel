// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let x: std.numeric.Int64 = 100;  // This will be captured
    // But the closure parameter shadows it
    let f = { (x: std.numeric.Int64) in x + 20 };
    // The parameter x (22) is used, not the captured x (100)
    if f(22) != 42 { return 1 }
    0
}
