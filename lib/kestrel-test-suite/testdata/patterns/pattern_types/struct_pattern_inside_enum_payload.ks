// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// A struct pattern can appear as an enum payload sub-pattern; `Name {`
// starts the struct pattern rather than reading `Name` as a payload label.

module Test

struct P {
    var x: Int64;
    var y: Int64;
}

func f(o: Optional[P]) -> Int64 {
    match o {
        .Some(P { x: 0, y }) => y,
        .Some(P { x, y }) => x + y,
        .None => -1
    }
}

@main
func main() -> lang.i64 {
    if f(.Some(P(x: 1, y: 2))) != 3 { return 1 }
    if f(.Some(P(x: 0, y: 7))) != 7 { return 2 }
    if f(.None) != -1 { return 3 }
    0
}
