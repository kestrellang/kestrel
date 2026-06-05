// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let pair = ("hello world", 42);
    let len = pair.0.byteCount;
    if len != 11 { return 1 }
    if pair.1 != 42 { return 2 }
    0
}
