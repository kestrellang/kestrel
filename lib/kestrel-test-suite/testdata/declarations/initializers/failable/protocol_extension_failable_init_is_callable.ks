// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

// A failable init defined in a protocol extension, delegating to the
// protocol's own failable requirement via `Self(...)`, must be callable on
// a conforming type and on a generic `T: P`. The concrete call
// (`Wrapper(doubling:)`) used to be E113: struct construction collected
// inits by hand and never looked in protocol extensions.

protocol Parseable {
    init(from source: Int64)?
}

extend Parseable {
    init(doubling source: Int64)? {
        guard let some v = Self(from: source * 2) else { return null }
        self = v
    }
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

func makeDoubled[T](doubling source: Int64) -> T? where T: Parseable {
    T(doubling: source)
}

@main
func main() -> lang.i64 {
    // Concrete call site
    match Wrapper(doubling: 21) {
        .Some(w) => { if w.value != 42 { return 1 } },
        _ => { return 2 }
    }
    match Wrapper(doubling: 0) {
        .Some(_) => { return 3 },
        _ => {}
    }

    // Generic call site
    let viaGeneric: Wrapper? = makeDoubled[Wrapper](doubling: 5);
    match viaGeneric {
        .Some(w) => { if w.value != 10 { return 4 } },
        _ => { return 5 }
    }

    0
}
