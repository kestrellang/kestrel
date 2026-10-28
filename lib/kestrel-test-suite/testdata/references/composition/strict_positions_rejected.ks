// test: diagnostics
// stdlib: true

// References 2b: Strict entries reject refs even inside nested type args —
// alias RHS and extension targets feed machinery with no ref story yet.
module Test

type Shortcut = Optional[&Int64] // ERROR(E485)

struct Wrap[T] where T: not Static {
    var v: T
}

extend Wrap[&Int64] { // ERROR(E485)
    func peek() -> Int64 {
        0
    }
}
