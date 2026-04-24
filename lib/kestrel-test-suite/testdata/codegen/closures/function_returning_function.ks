// test: execution
// stdlib: true

module Test

func add_one(x: std.num.Int64) -> std.num.Int64 {
    x + 1
}

func mul_two(x: std.num.Int64) -> std.num.Int64 {
    x * 2
}

func choose(flag: std.core.Bool) -> (std.num.Int64) -> std.num.Int64 {
    if flag {
        mul_two
    } else {
        add_one
    }
}

func main() -> lang.i64 {
    let f = choose(true);
    if f(21) != 42 { return 1 }
    0
}
