// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    var opt: std.num.Int64? = .Some(3);
    var sum: std.num.Int64 = 0;
    while let .Some(v) = opt {
        sum = sum + v;
        opt = .None;
    }
    if sum != 3 { return 1 }
    0
}
