// test: diagnostics
// stdlib: false

// `&mutating` pattern bindings need a MUTABLE scrutinee place (E210 —
// the E495 predicate family): a `let` scrutinee rejects.
module Test

enum Slot {
    case Filled(lang.i64)
    case Hole
}

func immutableScrutinee() {
    let s = Slot.Filled(1);
    let x = match s { // ERROR(E210)
        .Filled(&mutating v) => 1,
        .Hole => 0
    };
}

// A borrowed parameter is not a mutable place either.
func borrowedParam((&mutating a, b): (lang.i64, lang.i64)) { // ERROR(E210)
    a = 3;
}
