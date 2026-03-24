// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let a: std.core.Bool = false;
    let b: std.core.Bool = false;
    if a or b {
        1
    } else {
        0
    }
}
