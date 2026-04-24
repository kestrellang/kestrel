// test: diagnostics
// stdlib: true

module Test
import std.io.stdio.println

func main() -> lang.i64 {
    let some: std.num.Int64? = .Some(42);
    let _ = println(some.unwrap());
    0
}
