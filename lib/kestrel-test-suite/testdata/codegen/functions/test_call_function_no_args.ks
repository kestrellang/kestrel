// test: execution
// stdlib: true

module Test

func get_answer() -> std.numeric.Int64 {
    42
}

func main() -> lang.i64 {
    if get_answer() != 42 { return 1 }
    0
}
