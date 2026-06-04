// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

protocol Parseable {
    init(from source: Int64)?
}

struct Wrapper: Parseable {
    var value: Int64

    init(from source: Int64)? {
        if source == 0 {
            return null
        }
        self.value = source
    }
}

func make[T](from source: Int64) -> T? where T: Parseable {
    return T(from: source)
}

@main
func main() -> lang.i64 {
    let someOpt: Wrapper? = make[Wrapper](from: 42);
    match someOpt {
        .Some(w) => {
            if w.value != 42 { return 1 }
        },
        _ => { return 2 }
    }

    let none: Wrapper? = make[Wrapper](from: 0);
    match none {
        .Some(_) => { return 3 },
        _ => {}
    }

    0
}
