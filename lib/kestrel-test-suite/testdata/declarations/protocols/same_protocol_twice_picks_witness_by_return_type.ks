// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// A type conforming to one generic protocol twice has two same-shaped
// methods that differ only in return type. Each conformance must use its own
// (`Conv[String]` uses `-> String`). Both used to get the first one, and the
// `Int64` result was read as a `String` (segfault).

module Test

protocol Conv[T] { func conv() -> T }

struct S: Conv[Int64], Conv[String], Conv[Bool] {
    func conv() -> Int64 { 1 }
    func conv() -> String { "s" }
    func conv() -> Bool { true }
}

func asString[X](x: X) -> String where X: Conv[String] { x.conv() }
func asInt[X](x: X) -> Int64 where X: Conv[Int64] { x.conv() }
func asBool[X](x: X) -> Bool where X: Conv[Bool] { x.conv() }

@main
func main() -> lang.i64 {
    if asString(S()) != "s" { return 1 }
    if asInt(S()) != 1 { return 2 }
    if not asBool(S()) { return 3 }
    let direct: String = S().conv();
    if direct != "s" { return 4 }
    0
}
