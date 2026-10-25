// test: execution
// stdlib: true
// expect-stdout: int:7 two:z\n

// G17 S4, permit side: filtering a projection's bound search by receiver must
// keep every bound that DOES name this receiver. `A.Item: Show` still grants
// `show` to `self.a.produce()`. And when each receiver has its own protocol
// supplying a same-named `show` (`A.Item: Show`, `B.Item: Show2`), each call
// must find exactly its own — the unfiltered search saw both protocols on
// both receivers.
//
// Pairs with `assoc_projection_bound_member_other_receiver.ks`: that file
// proves the filter rejects a foreign receiver's bound; this one proves it
// is not a blanket rejection of projection-bounded members.

module Test

import std.text.String
import std.numeric.Int64

protocol Show { func show() -> String }
protocol Show2 { func show() -> String }
extend Int64: Show { public func show() -> String { "int:\(self)" } }
extend String: Show2 { public func show() -> String { "two:\(self)" } }

protocol Producer { type Item; func produce() -> Item }

struct IntSrc { var v: Int64; }
extend IntSrc: Producer {
    public type Item = Int64;
    public func produce() -> Int64 { self.v }
}
struct StrSrc { var s: String; }
extend StrSrc: Producer {
    public type Item = String;
    public func produce() -> String { self.s.clone() }
}

struct Pair[A, B] where A: Producer, B: Producer, A.Item: Show, B.Item: Show2 {
    var a: A;
    var b: B;
    public func go() -> String { "\(self.a.produce().show()) \(self.b.produce().show())" }
}

@main
func main() -> lang.i32 {
    print(Pair(a: IntSrc(v: 7), b: StrSrc(s: "z")).go());
    0
}
