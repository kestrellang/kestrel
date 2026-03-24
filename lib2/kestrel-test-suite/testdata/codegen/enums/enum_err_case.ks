// test: execution
// stdlib: true

module Test

enum Result {
    case Ok(value: std.num.Int64)
    case Err(code: std.num.Int64)
}

func handle(r: Result) -> std.num.Int64 {
    match r {
        .Ok(value: v) => v,
        .Err(code: c) => c + 100
    }
}

func main() -> lang.i64 {
    let err = Result.Err(code: 10);
    // code (10) + 100 = 110
    if handle(err) != 110 { return 1 }
    0
}
