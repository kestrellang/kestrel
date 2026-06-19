// test: diagnostics
// stdlib: false

// Inside `extend &T where T: Probe`, `self` types as the REF (`&T`), never
// the synthetic `lang.&` entity — so the transparent-place receiver peel
// fires and `self.probe()` dispatches on the POINTEE through the where
// bound. (If self leaked as the entity, member resolution would find the
// extension's own members and bodies like a forwarding `probed()` would
// recurse.) Bodies here only need to TYPE — the methods are reachable
// solely via witness dispatch, which lands in later commits.
module Main

protocol Probe { func probe() -> lang.i64 }

extend &T where T: Probe {
    func probed() -> lang.i64 { self.probe() }
    func same(other: Self) -> lang.i64 { other.probe() }
}

extend &mutating T where T: Probe {
    func probedMut() -> lang.i64 { self.probe() }
}

@main
func main() {
}
