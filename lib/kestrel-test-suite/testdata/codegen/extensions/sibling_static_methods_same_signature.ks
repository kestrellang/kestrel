// test: execution
// stdlib: false
// expect-exit: 0

// Regression: two `extend X { static func foo(...) }` blocks for different
// X's, in the same module, with identical signatures used to collide at
// codegen/link time. `qualified_name` skipped the Extension in the parent
// chain (extensions carry no `Name`), so both `makeDefault`s below were
// named `Test.makeDefault` and — since they share a signature and static
// methods pass no `self_type` to the mangler — produced the same mangled
// symbol. Fix injects the extended type's path segments into the qualified
// name, yielding `Test.A.makeDefault` and `Test.B.makeDefault`.

module Test

struct A {
    public var v: lang.i64
    public init(v: lang.i64) { self.v = v }
}

struct B {
    public var v: lang.i64
    public init(v: lang.i64) { self.v = v }
}

extend A {
    public static func makeDefault() -> lang.i64 { 1 }
}

extend B {
    public static func makeDefault() -> lang.i64 { 2 }
}

@main
func main() -> lang.i64 {
    // Return non-zero iff either static method returns the wrong value.
    // Sum them and subtract the expected total (1 + 2 = 3); 0 on success.
    lang.i64_sub(lang.i64_add(A.makeDefault(), B.makeDefault()), 3)
}
