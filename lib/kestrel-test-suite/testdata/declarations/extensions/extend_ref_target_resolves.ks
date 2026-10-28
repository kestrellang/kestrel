// test: diagnostics
// stdlib: false

// `&T` / `&mutating T` are extension targets: they resolve to the synthetic
// generic `lang.&` / `lang.&mutating` entities (the `extend ():` / `extend !:`
// precedent, generalized with a pointee type param). The pointee name binds
// by NAME against the entity's declared param — so it is spelled `T`, like
// `extend Optional[T]` echoes Optional's declared name. This pins target
// resolution, the extension-scoped `T`, where clauses, and member lowering
// (including the `other: Self` param, whose top-level ref comes from Self
// substitution — a written `&T` param stays E480). Conformance dispatch and
// witnesses land in later commits.
module Main

protocol Probe { func probe() -> lang.i64 }

extend &T {
    func tagged() -> lang.i64 { 4 }
}

extend &T where T: Probe {
    func same(other: Self) -> lang.i64 { 6 }
}

extend &mutating T {
    func taggedMut() -> lang.i64 { 5 }
}

@main
func main() {
}
