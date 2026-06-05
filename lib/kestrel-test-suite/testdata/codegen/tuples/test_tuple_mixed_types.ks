// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let t = (true, 42);
    if t.1 != 42 { return 1 }
    0
}
