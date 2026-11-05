// test: diagnostics
// stdlib: true

// A type that opts out of Copyable accepts any argument for its params, so
// inside its own methods those params may be non-Copyable: using a payload
// twice is a use after move, not a silent bit-copy.

module Test

enum Box[T]: not Copyable {
    case Full(T)
    case Empty
}

extend Box[T] {
    consuming func twice() -> (T, T) {
        match self {
            .Full(x) => {
                let a = x;
                let b = x; // ERROR: use of moved value
                (a, b)
            },
            .Empty => fatalError("empty")
        }
    }
}
