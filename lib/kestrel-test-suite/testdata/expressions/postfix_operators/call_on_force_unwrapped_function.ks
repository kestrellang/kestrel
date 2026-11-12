// test: execution
// stdlib: true
// expect-exit: 0

module Test

func inc(x: Int64) -> Int64 { x + 1 }

// `f!(2)` calls the unwrapped function. `let g = f!; g(2)` works, but the
// direct call fails today: post-mono verification finds `forceUnwrap`
// dispatched on the function type itself instead of on the Optional.
@main
func main() -> lang.i64 {
    let f: (escaping (Int64) -> Int64)? = .Some(inc);
    if f!(2) != 3 { return 1 }
    0
}
