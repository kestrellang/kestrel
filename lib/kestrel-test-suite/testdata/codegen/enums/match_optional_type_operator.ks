// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let opt: std.numeric.Int64? = .Some(7);
    let val = match opt {
        .Some(v) => v,
        .None => 0
    };
    if val != 7 { return 1 }
    0
}
