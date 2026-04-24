// test: execution
// stdlib: true

module Test

struct Wrapper[T] {
    let value: T
}

enum MaybeProvider[T] {
    case Provider(make: () -> Wrapper[T])
    case Empty
}

func main() -> lang.i64 {
    let provider = MaybeProvider[std.num.Int64].Provider(
        make: { Wrapper[std.num.Int64](value: 42) }
    );

    match provider {
        .Provider(make: m) => {
            let w = m();
            if w.value != 42 { return 1 }
        },
        .Empty => { return 2 }
    }

    0
}
