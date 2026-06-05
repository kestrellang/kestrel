// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let pair = ("hello", "world");
    if pair.0.isEqual(to: "hello") != true { return 1 }
    if pair.1.isEqual(to: "hello") != false { return 2 }
    0
}
