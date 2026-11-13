// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: Pair T\nWithString F\nUserClone F\nEnum T\nInt64? T\nString? F\nRawPointer T\nTuple T\nArray F\n

// `lang.is_bitwise[T]()` is true when copying a `T` copies its bytes and
// nothing else. A struct whose memberwise clone only copies bitwise fields
// counts; a user `clone()`, or a field that clones, does not.
module Test

import std.memory.(RawPointer)

struct Pair { var a: Int64; var b: Int64 }

struct WithString { var a: Int64; var s: String }

struct UserClone: Cloneable {
    var a: Int64
    func clone() -> UserClone { UserClone(a: self.a) }
}

enum Shape {
    case Dot(Int64)
    case Line(Pair)
}

func show(name: String, b: lang.i1) {
    if b { print("\(name) T\n") } else { print("\(name) F\n") }
}

@main
func main() {
    show("Pair", lang.is_bitwise[Pair]());
    show("WithString", lang.is_bitwise[WithString]());
    show("UserClone", lang.is_bitwise[UserClone]());
    show("Enum", lang.is_bitwise[Shape]());
    show("Int64?", lang.is_bitwise[Int64?]());
    show("String?", lang.is_bitwise[String?]());
    show("RawPointer", lang.is_bitwise[RawPointer]());
    show("Tuple", lang.is_bitwise[(Pair, Float64)]());
    show("Array", lang.is_bitwise[Array[Int64]]());
}
