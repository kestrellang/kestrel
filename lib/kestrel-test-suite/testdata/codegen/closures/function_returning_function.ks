// test: execution
// stdlib: true

module Test

func add_one(x: std.numeric.Int64) -> std.numeric.Int64 {
    x + 1
}

func mul_two(x: std.numeric.Int64) -> std.numeric.Int64 {
    x * 2
}

func choose(flag: std.core.Bool) -> (std.numeric.Int64) -> std.numeric.Int64 {
    if flag {
        mul_two
    } else {
        add_one
    }
}

@main
func main() -> lang.i64 {
    let f = choose(true);
    if f(21) != 42 { return 1 }
    0
}
