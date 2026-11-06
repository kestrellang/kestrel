// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// A match whose arms test struct fields switches on those fields by name.
// The decision tree used to address struct children as tuple elements,
// which failed in codegen ("TupleExtract on non-tuple").

module Test

struct P {
    var x: Bool;
    var y: Bool;
}

func classify(p: P) -> Int64 {
    match p {
        P { x: true, y: _ } => 1,
        P { x: false, y: true } => 2,
        P { x: false, y: false } => 3
    }
}

@main
func main() -> lang.i64 {
    if classify(P(x: true, y: false)) != 1 { return 1 }
    if classify(P(x: false, y: true)) != 2 { return 2 }
    if classify(P(x: false, y: false)) != 3 { return 3 }
    0
}
