// test: execution
// stdlib: true

module Test

func get_answer() -> std.num.Int64 {
    42
}

func call_it(f: () -> std.num.Int64) -> std.num.Int64 {
    f()
}

func main() -> lang.i64 {
    if call_it(get_answer) != 42 { return 1 }
    0
}
