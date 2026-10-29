// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

// A non-failable init may satisfy a failable protocol requirement (as in
// Swift): calling it through the requirement yields `.Some(value)`. The
// witness is an adapter that runs the init and returns `.Some(())`; without
// it the plain `()` return was read as an Optional (SIGSEGV at -O0).

protocol Parseable {
    init(from source: Int64)?
}

extend Parseable {
    static func parse(source: Int64) -> Self? {
        Self(from: source)
    }
}

// Always succeeds, so it declares a plain init.
struct Always: Parseable {
    var value: Int64

    init(from source: Int64) {
        self.value = source
    }
}

func make[T](from source: Int64) -> T? where T: Parseable {
    T(from: source)
}

@main
func main() -> lang.i64 {
    // Direct call keeps the non-failable signature
    let direct = Always(from: 7);
    if direct.value != 7 { return 1 }

    // Through the requirement from a protocol extension
    match Always.parse(9) {
        .Some(a) => { if a.value != 9 { return 2 } },
        _ => { return 3 }
    }

    // Through the requirement from a generic function
    let viaGeneric: Always? = make[Always](from: 11);
    match viaGeneric {
        .Some(a) => { if a.value != 11 { return 4 } },
        _ => { return 5 }
    }

    0
}
