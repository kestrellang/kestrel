// test: execution
// stdlib: true

module Test

func add(a: Int64, b: Int64) -> Int64 {
    a + b
}

func main() -> lang.i64 {
    let pair = (20, 22);
    let result = add(pair.0, pair.1);
    if result != 42 { return 1 }
    0
}
