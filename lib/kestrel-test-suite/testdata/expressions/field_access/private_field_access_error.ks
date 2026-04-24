// test: diagnostics
// stdlib: false

module Main

struct Secret {
    private let hidden: lang.i64
}

func peek(s: Secret) -> lang.i64 {
    s.hidden // ERROR: is private
}
