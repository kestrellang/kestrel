// test: execution
// stdlib: true
// expect-exit: 0

// `x |> f` is `f(x)`; a chain reads left to right.

module Test

func double(x: Int64) -> Int64 { x * 2 }
func inc(x: Int64) -> Int64 { x + 1 }
func square(x: Int64) -> Int64 { x * x }

@main
func main() -> lang.i64 {
    if (3 |> double) != 6 { return 1 }
    // g(f(x)), not f(g(x))
    if (3 |> double |> inc) != 7 { return 2 }
    if (3 |> inc |> double |> square) != 64 { return 3 }
    0
}
