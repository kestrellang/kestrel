// test: execution
// stdlib: true
// expect-exit: 0

// A line may start with `|>`: the binary-operator loop ignores newlines.

module Test

func double(x: Int64) -> Int64 { x * 2 }
func inc(x: Int64) -> Int64 { x + 1 }

@main
func main() -> lang.i64 {
    let r = 5
        |> double
        |> inc;
    if r != 11 { return 1 }
    0
}
