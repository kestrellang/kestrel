// test: execution
// stdlib: true
// expect-exit: 0

// G13 / Bug 1, `Cloneable` twin of
// `protocol_ext_witness_gated_by_copyable_bound.ks`. Third member of the
// family whose control is `constrained_protocol_ext_witness.ks`
// (`where T: Equatable`) — one program shape, three bound protocols, one
// behavior.
//
// The concrete binding is deliberately NOT `Int64`. `Int64` is *Copyable*, and
// the two builtins are distinct rungs of the same ladder:
//   Copyable  holds when  semantics != NotCopyable   (Copyable OR Cloneable)
//   Cloneable holds when  semantics == Cloneable     (strictly)
// so `Int64` satisfies `T: Copyable` but NOT `T: Cloneable`, and reusing it
// here would assert the wrong thing. `Res` declares `Cloneable` and owns a
// user-written `clone()`, giving it `CopySemantics::Cloneable`.
//
// The negative half of this bound — `Box[Int64]` must NOT pick up a member
// from `extend Box[T] where T: Cloneable` — is the same enforcement the
// stdlib tests `stdlib/rcbox/get_value_rejects_noncopyable_payload.ks` and
// `stdlib/memory/pointee_rejects_noncopyable_payload.ks` pin for `Copyable`.

module Test

import std.numeric.Int64

protocol Dup {
    func dup() -> Self
}

protocol Container[T] {
    func item() -> T
}

extend Container[T] where T: Cloneable {
    public func dup() -> Self { self }
}

// Cloneable, and therefore NOT plain-Copyable: an explicit conformance plus a
// user-written `clone()` classifies as `CopySemantics::Cloneable`.
struct Res: Cloneable {
    var id: Int64;
    func clone() -> Res { Res(id: self.id) }
}

struct BoxR: Container[Res] {
    var v: Res;
    func item() -> Res { self.v.clone() }
}

extend BoxR: Dup { }

func dupG[T](value: T) -> T where T: Dup { value.dup() }

@main
func main() -> lang.i32 {
    let a = BoxR(v: Res(id: 9));

    let b = a.dup();
    if b.item().id != 9 { return 10 }

    let c = dupG(BoxR(v: Res(id: 12)));
    if c.item().id != 12 { return 11 }

    0
}
