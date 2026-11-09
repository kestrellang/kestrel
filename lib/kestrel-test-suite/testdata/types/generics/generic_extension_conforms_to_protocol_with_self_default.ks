// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: 5 7\n

// A generic type conforming in an extension to a protocol whose param
// defaults to `Self` (`Addable[Other = Self]`): bare `Addable` means
// `Addable[W[A]]`, not the implicit-positional `Addable[A]`, and spelled
// out it must still find its witness after monomorphization (the witness
// arg `W[A]` used to be compared to `W[D]` by identity).
module Test

import std.core.(Addable, Defaultable)

struct D: Defaultable { init() {} }

struct W[A = D] { var n: Int64 }
extend W[A]: Addable where A: Defaultable {
    type Output = W[A]
    public static var zero: W[A] { get { W[A](n: 0) } }
    public consuming func add(consuming other: W[A]) -> W[A] { W[A](n: self.n + other.n) }
}

struct V[A] { var n: Int64 }
extend V[A]: Addable[V[A]] {
    type Output = V[A]
    public static var zero: V[A] { get { V[A](n: 0) } }
    public consuming func add(consuming other: V[A]) -> V[A] { V[A](n: self.n + other.n) }
}

@main
func main() {
    let w = W(n: 2) + W(n: 3);
    let v = V[D](n: 3) + V[D](n: 4);
    print("\(w.n) \(v.n)\n");
}
