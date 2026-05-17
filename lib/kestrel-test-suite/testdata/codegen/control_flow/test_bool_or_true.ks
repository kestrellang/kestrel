// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let a: std.core.Bool = false;
    let b: std.core.Bool = true;
    if a or b {
        0
    } else {
        1
    }
}
