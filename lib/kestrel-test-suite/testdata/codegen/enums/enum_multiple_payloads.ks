// test: execution
// stdlib: false

module Test

enum Result {
    case Ok(value: lang.i64)
    case Err(code: lang.i64)
}

func handle(r: Result) -> lang.i64 {
    match r {
        .Ok(value: v) => v,
        .Err(code: c) => lang.i64_add(c, 100)
    }
}

@main
func main() -> lang.i64 {
    let ok = Result.Ok(value: 42);
    if lang.i64_eq(handle(ok), 42) { 0 } else { 1 }
}
