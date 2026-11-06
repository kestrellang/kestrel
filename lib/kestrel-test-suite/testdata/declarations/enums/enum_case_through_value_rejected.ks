// test: diagnostics

// An enum case is reached through the enum type (`E.b`, `Optional[Int64].Some`),
// never through a value: `E.a.b` used to type-check.

module Test

enum E {
    case a
    case b
}

func use() {
    let ok = E.b;
    let viaType = Optional[Int64].Some(3);
    let bad = E.a.b; // ERROR: 'b' is a static member and cannot be used on an instance
}
