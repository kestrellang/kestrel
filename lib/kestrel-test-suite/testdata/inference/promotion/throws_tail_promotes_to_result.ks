// test: execution
// stdlib: true
// expect-exit: 0

// bidi ruling D7(i): a throwing function's tail is a check leaf against its
// `Result` return type, so a bare success value promotes to `.Ok`, exactly as
// `let r: Int64 throws E = 42` already did. Before P3 the tail was rejected
// ("expected Result[Int64, E] got Int64").

module Test

enum Oops {
    case Bad
}

func twice(n: Int64) -> Int64 throws Oops { n * 2 }

@main
func main() -> lang.i64 {
    match twice(21) {
        .Ok(v) => if v == 42 { 0 } else { 1 },
        .Err(_) => 2,
    }
}
