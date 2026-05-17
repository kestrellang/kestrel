// test: execution
// stdlib: true

module Test

struct Provider[T] {
    let provide: () -> T
}

func main() -> lang.i64 {
    let p = Provider[std.numeric.Int64](provide: { 42 });
    if (p.provide)() != 42 { return 1 }
    0
}
