// test: execution
// stdlib: true

module Test

struct Config {
    let multiplier: std.numeric.Int64
}

func main() -> lang.i64 {
    let config = Config(multiplier: 2);
    let f = { (x: std.numeric.Int64) in x * config.multiplier };
    if f(21) != 42 { return 1 }
    0
}
