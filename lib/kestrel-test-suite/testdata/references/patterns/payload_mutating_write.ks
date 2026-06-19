// test: execution
// stdlib: false
// backends: cranelift,llvm

// `&mutating v` pattern binding: writes through the binding land in the
// enum payload IN PLACE (the scrutinee is a var — a mutable place; the
// match pins its address and the arm stores through the projection).
module Test

enum Slot {
    case Filled(lang.i64)
    case Hole
}

@main
func main() -> lang.i64 {
    var s = Slot.Filled(5);
    match s {
        .Filled(&mutating v) => {
            v = 99;
        },
        .Hole => { }
    };
    let check = match s {
        .Filled(x) => x,
        .Hole => 0
    };
    if lang.i64_eq(check, 99) { } else { return 1; }
    0
}
