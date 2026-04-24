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

func main() -> lang.i64 {
    let err = Result.Err(code: 10);
    // code (10) + 100 = 110
    if lang.i64_eq(handle(err), 110) { 0 } else { 1 }
}
