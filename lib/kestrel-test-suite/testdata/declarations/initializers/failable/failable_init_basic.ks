// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

struct Wrapper {
    var value: Int64

    init(from source: Int64)? {
        if source == 0 {
            return null
        }
        self.value = source
    }
}

@main
func main() -> lang.i64 {
    let someOpt = Wrapper(from: 42);
    match someOpt {
        .Some(w) => {
            if w.value != 42 { return 1 }
        },
        _ => { return 2 }
    }

    let none = Wrapper(from: 0);
    match none {
        .Some(_) => { return 3 },
        _ => {}
    }

    0
}
