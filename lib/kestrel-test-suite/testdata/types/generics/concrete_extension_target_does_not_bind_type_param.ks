// test: diagnostics
// stdlib: false
module Test

// `extend Box[Payload]` binds nothing — the LHS supplies a concrete argument, so
// Box's parameter name `T` must NOT leak into the extension body. Extension
// type-param lookup is scoped to the names the LHS actually introduces, not to
// every parameter the target nominal happens to declare.

struct Payload {}

struct Box[T] { var value: T }

extend Box[Payload] {
    func leak(v: T) {} // ERROR: T
}
