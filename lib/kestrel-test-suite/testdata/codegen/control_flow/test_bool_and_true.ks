// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let a: std.core.Bool = true;
    let b: std.core.Bool = true;
    if a and b {
        0
    } else {
        1
    }
}
