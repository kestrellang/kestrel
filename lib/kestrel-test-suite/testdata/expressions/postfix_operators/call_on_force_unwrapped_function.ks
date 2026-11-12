// test: execution
// stdlib: true
// expect-exit: 0

module Test

func inc(x: Int64) -> Int64 { x + 1 }

// `f!(2)` calls the unwrapped function. The outer call must not reuse the
// inner `forceUnwrap` resolution as its callee: that dispatched
// `forceUnwrap` on the function type and failed post-mono verification.
@main
func main() -> lang.i64 {
    let f: (escaping (Int64) -> Int64)? = .Some(inc);
    if f!(2) != 3 { return 1 }
    0
}
