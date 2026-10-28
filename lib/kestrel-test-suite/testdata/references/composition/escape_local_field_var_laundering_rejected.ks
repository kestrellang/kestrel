// test: diagnostics
// stdlib: true

// G1 closure, field-projection variant: a ref minted from a FIELD of a
// function-local `var` roots at the local's slot (the field address
// inherits the slot's root, not a fresh temp), so the var-slot roundtrip
// still cannot launder the escape taint.
module Test

struct Box {
    var v: Int64
}

func launder() -> Optional[&Int64] {
    var x = Box(v: 1);
    let r = &x.v;
    var o: Optional[&Int64] = .Some(r);
    o // ERROR(E494)
}
