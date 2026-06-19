// test: execution
// backends: cranelift,llvm
// stdlib: true

// Stage 2d: TRIVIAL member type aliases — the assoc-type binding shape —
// may be a bare ref (`type Item = &T`, `&mutating T`) or carry refs in
// aggregate positions (`Optional[&T]`). All three homes form: struct
// body, enum body, extension. Named uses are eagerly expanded at the use
// site, so a `-> Item` return behaves exactly like a written `-> &T`
// (and illegal positions reject — see assoc_ref_alias_use_site_rejected).
module Test

struct Holder[T] {
    type Item = &T
    type MutItem = &mutating T
    type MaybeItem = Optional[&T]
    var v: T

    func view() -> Item {
        self.v
    }

    mutating func slot() -> MutItem {
        self.v
    }
}

enum Cell[T] {
    case Empty
    case Full(T)
    type Item = &T
}

struct Plain {
    var v: Int64
}

extend Plain {
    type View = &Int64

    func look() -> View {
        self.v
    }
}

@main
func main() -> lang.i64 {
    var h = Holder(v: 5);
    if h.view() != 5 { return 1; }
    h.slot() = 9;
    if h.view() != 9 { return 2; }
    if h.v != 9 { return 3; }
    let p = Plain(v: 4);
    if p.look() != 4 { return 4; }
    0
}
