// test: execution
// stdlib: true

module Test

enum MaybeTransform {
    case Just(f: (std.numeric.Int64) -> std.numeric.Int64)
    case Nothing
}

func main() -> lang.i64 {
    let mt = MaybeTransform.Just(f: { (x) in x + 32 });
    let result = match mt {
        .Just(f: f) => f(10),
        .Nothing => 0
    };
    if result != 42 { return 1 }
    0
}
