// test: diagnostics
// stdlib: true

// Binding an `Optional[&T]` copies the payload out (see
// `binding_copies_out.ks`), so a `not Copyable` payload can't be bound:
// it would move out of the borrowed place (E503). Spell the binding
// `Optional[&T]` to keep the reference instead.
module Test

struct NC: not Copyable {
    var n: Int64
}

struct B {
    var x: NC

    var first?: NC {
        ref {
            let r = &self.x;
            .Some(r)
        }
    }
}

func use(b: B) {
    let f = b.first; // ERROR(E503)
    let kept: Optional[&NC] = b.first;
}
