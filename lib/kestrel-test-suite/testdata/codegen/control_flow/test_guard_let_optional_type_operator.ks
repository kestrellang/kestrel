// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let opt: std.num.Int64? = .Some(1);
    guard let .Some(v) = opt else {
        return 1
    }
    if v != 1 { return 1 }
    0
}
