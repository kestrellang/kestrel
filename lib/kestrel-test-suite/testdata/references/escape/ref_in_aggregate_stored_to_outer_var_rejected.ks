// test: diagnostics

// A value carrying `&x` (here inside a struct inside an Optional) stored into
// a `var` declared outside `x`'s scope would dangle once that scope ends. It
// used to be accepted and read freed stack memory. Storing it into a binding
// in the same scope is fine.

module Test

struct Cur {
    var item: &Int64;
}

func escapes() -> Int64 {
    var c: Optional[Cur] = .None;
    if true {
        let x = 5;
        let r = &x;
        c = .Some(Cur(item: r)); // ERROR: a reference to 'x' is stored somewhere that outlives 'x'
    }
    0
}

func sameScope() -> Int64 {
    let x = 5;
    let r = &x;
    var c: Optional[Cur] = .None;
    c = .Some(Cur(item: r));
    if let .Some(cur) = c { cur.item } else { 0 }
}
