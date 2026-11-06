// test: execution
// stdlib: false
// backends: cranelift,llvm

// `&` / `&mutating` binders work in every pattern position, not only
// `match` arms: if-let, while-let, guard-let, `let` destructures of a
// place, parameter destructures and closure parameters. Writes through a
// `&mutating` binder land in the matched place.
module Test

enum Slot {
    case Filled(lang.i64)
    case Hole
}

func read(s: Slot) -> lang.i64 {
    match s {
        .Filled(x) => x,
        .Hole => 0
    }
}

func guardBump(mutating s: Slot) -> lang.i64 {
    guard let .Filled(&mutating v) = s else { return 0; }
    v = lang.i64_add(v, 10);
    v
}

func sumPair((&a, &b): (lang.i64, lang.i64)) -> lang.i64 {
    lang.i64_add(a, b)
}

@main
func main() -> lang.i64 {
    // if-let
    var s = Slot.Filled(1);
    if let .Filled(&mutating v) = s {
        v = 2;
    }
    if lang.i64_eq(read(s), 2) { } else { return 1; }
    if let .Filled(&v) = s {
        if lang.i64_eq(v, 2) { } else { return 2; }
    }

    // while-let
    var n = Slot.Filled(0);
    var steps = 0;
    while let .Filled(&mutating v) = n {
        v = lang.i64_add(v, 1);
        steps = lang.i64_add(steps, 1);
        if lang.i64_eq(steps, 3) { n = Slot.Hole; }
    }
    if lang.i64_eq(steps, 3) { } else { return 3; }

    // guard-let
    var g = Slot.Filled(5);
    if lang.i64_eq(guardBump(g), 15) { } else { return 4; }
    if lang.i64_eq(read(g), 15) { } else { return 5; }

    // let destructure of a place
    var t = (1, 2);
    let (&mutating x, y) = t;
    x = lang.i64_add(x, y);
    if lang.i64_eq(t.0, 3) { } else { return 6; }

    // parameter and closure-parameter destructures
    if lang.i64_eq(sumPair((4, 5)), 9) { } else { return 7; }
    let f = { ((&a, b): (lang.i64, lang.i64)) in lang.i64_add(a, b) };
    if lang.i64_eq(f((1, 1)), 2) { } else { return 8; }
    0
}
