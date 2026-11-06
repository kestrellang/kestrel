// test: execution
// stdlib: true
// expect-exit: 0

module Test

// Static computed properties through `Self` and an explicit owner, where the
// getter body uses the owner's param.

protocol Sized { static var size: Int64 { get } }
struct Four: Sized { static var size: Int64 { 4 } }

struct Box[T] where T: Sized {
    static var doubled: Int64 { T.size * 2 }
    static func viaSelf() -> Int64 { Self.doubled }
}

@main
func main() -> Int64 {
    if Box[Four].doubled != 8 { return 1 }
    if Box[Four].viaSelf() != 8 { return 2 }
    0
}
