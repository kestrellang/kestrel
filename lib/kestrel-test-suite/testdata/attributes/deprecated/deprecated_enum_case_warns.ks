// test: diagnostics
// stdlib: false

module Test

enum Mode {
    case Fast
    @deprecated("use '.Fast'")
    case Quick
}

func pick() -> Mode {
    .Quick // WARN: 'Mode.Quick' is deprecated: use '.Fast'
}

func qualified() -> Mode {
    Mode.Quick // WARN: 'Mode.Quick' is deprecated
}

func check(m: Mode) -> lang.i64 {
    match m {
        .Quick => 1, // WARN: 'Mode.Quick' is deprecated
        _ => 0,
    }
}
