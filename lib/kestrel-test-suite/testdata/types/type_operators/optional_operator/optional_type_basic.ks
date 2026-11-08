// test: diagnostics
// stdlib: true

module Test
import std.streams.println

func main() -> lang.i64 {
    let someOpt: std.numeric.Int64? = .Some(42);
     println(someOpt.unwrap());
    0
}
