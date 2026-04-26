// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let t = (42, 0);
    if t.0 != 42 { return 1 }
    0
}
