// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

// Inside a protocol extension, a static VAR requirement read through `Self`
// and passed straight into another `Self.` static call must compile. As an
// argument it used to lower as a stored global's address (`global_ref` to the
// requirement), failing in the backend with "global entity not found in
// statics"; it reads through the witness getter like every other use.

protocol Sized {
    static var size: Int64 { get }
}

extend Sized {
    static func sized(count count: Int64) -> Int64? {
        if count < 0 { return null }
        count * 2
    }

    static func defaultSized() -> Int64 {
        Self.sized(count: Self.size).expect("size is valid")
    }
}

struct Small: Sized {
    static var size: Int64 { 12 }
}

@main
func main() -> lang.i64 {
    if Small.defaultSized() != 24 { return 1 }
    0
}
