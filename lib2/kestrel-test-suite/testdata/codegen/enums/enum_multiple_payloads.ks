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
    let ok = Result.Ok(value: 42);
    if handle(ok) != 42 { return 1 }
    0
}
