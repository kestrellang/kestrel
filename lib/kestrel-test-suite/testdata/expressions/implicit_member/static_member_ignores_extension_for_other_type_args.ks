// test: execution
// stdlib: true
// expect-exit: 0
//
// A static member from an extension that only applies to OTHER type arguments
// (`extend Wrap[Bool, Bool]`) must not be a candidate for `.make` on
// `Wrap[String, String]`. Today static/implicit member lookup is keyed on the
// type entity only, so both `make`s are candidates and `.make("x")` fails with
// E104 "not found on _" (the return type is also poisoned); the explicit
// `Wrap[String, String].make("y")` gives a false E116 "static member ... on an
// instance". This is what made a second `FromResidual` conformance on `Result`
// break every `try` in the program.
module Test

enum Wrap[T, E] { case A(T) case B(E) }

extend Wrap[T, E] {
    static func make(consuming e: E) -> Wrap[T, E] { .B(e) }
}
extend Wrap[Bool, Bool] {
    static func make(consuming e: Int64) -> Wrap[Bool, Bool] { .A(e > 0) }
}

func implicitOwner() -> Wrap[String, String] {
    return .make("x")
}
func explicitOwner() -> Wrap[String, String] {
    return Wrap[String, String].make("y")
}

@main
func main() -> Int64 {
    let a = match implicitOwner() { .A(_) => 1, .B(_) => 0 };
    let b = match explicitOwner() { .A(_) => 1, .B(_) => 0 };
    a + b
}
