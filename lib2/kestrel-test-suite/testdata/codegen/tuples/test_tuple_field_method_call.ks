// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let pair = ("hello", "world");
    if pair.0.equals("hello") != true { return 1 }
    if pair.1.equals("hello") != false { return 2 }
    0
}
