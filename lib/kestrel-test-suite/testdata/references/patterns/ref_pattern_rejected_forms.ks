// test: diagnostics
// stdlib: false

// `&mutating` pattern bindings need a MUTABLE scrutinee place (E210 —
// the E495 predicate family): a `let` scrutinee rejects. `&` binders
// outside match arms stay E211 (if-let here; let-destructure pinned in
// ref_pattern_positions_rejected.ks).
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

func ifLetPosition() {
    let s = Slot.Filled(1);
    if let .Filled(&v) = s { // ERROR(E211)
    }
}
