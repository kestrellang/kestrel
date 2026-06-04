// test: execution
// stdlib: true

module Test

func get_answer() -> std.numeric.Int64 {
    42
}

func call_it(f: () -> std.numeric.Int64) -> std.numeric.Int64 {
    f()
}

@main
func main() -> lang.i64 {
    if call_it(get_answer) != 42 { return 1 }
    0
}
