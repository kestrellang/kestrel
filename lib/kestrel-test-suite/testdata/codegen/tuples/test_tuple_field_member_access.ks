// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let pair = ("hello", "world");
    if pair.0.byteCount != 5 { return 1 }
    if pair.1.byteCount != 5 { return 2 }
    0
}
