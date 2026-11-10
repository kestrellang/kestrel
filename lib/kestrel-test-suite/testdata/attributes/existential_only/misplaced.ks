// test: diagnostics
// stdlib: true

// `@existentialOnly` only means something on a default, in `extend P`, for
// one of `P`'s requirements.

module Test

protocol Q {
    func f() -> Int64
}

extend Q {
    @existentialOnly // ERROR: '@existentialOnly' needs a default for a requirement of the protocol it extends
    func g() -> Int64 { 1 }
}

struct S: Q {
    func f() -> Int64 { 2 }
    @existentialOnly // ERROR: '@existentialOnly' needs a default for a requirement of the protocol it extends
    func h() -> Int64 { 3 }
}
