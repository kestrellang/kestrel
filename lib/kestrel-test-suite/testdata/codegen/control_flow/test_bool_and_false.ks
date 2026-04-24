// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let a: std.core.Bool = true;
    let b: std.core.Bool = false;
    if a and b {
        1
    } else {
        0
    }
}
