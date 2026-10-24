// test: diagnostics
// stdlib: false

module Test

@nonExhaustive
public enum Mode {
    case Fast
    case Slow
}

func pick(m: Mode) -> lang.i64 {
    match m {
        .Fast => 1,
        .Slow => 2,
    }
}
