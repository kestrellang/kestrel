// test: execution
// stdlib: true

// `null` is sugar for `ExpressibleByNullLiteral.init()`. MIR-lowering of the
// literal must invoke that init so the destination slot is properly written
// (e.g. `Optional<T>::None`). When the lowering instead returned a zero-byte
// unit immediate, the slot was left uninitialized and the downstream `match`
// read a garbage discriminant. With a refcounted payload (here `String`),
// the wrong-arm destructure walked a null `RcBox` pointer and SIGSEGV'd.
// Originally surfaced as an intermittent crash in `flock build`.

module Test

enum Body: std.core.Cloneable {
    case Text(std.text.String)
    case Empty
}

extend Body {
    func clone() -> Body {
        match self {
            .Text(s) => Body.Text(s.clone()),
            .Empty => Body.Empty
        }
    }
}

@main
func main() -> lang.i64 {
    let b: Body? = null;
    match b {
        .Some(_) => 1,
        .None => 0
    }
}
