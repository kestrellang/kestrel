// test: diagnostics
// stdlib: false

// G12: `foo` has type `!`, so evaluating it never produces a value and the
// `let x` initializer never completes — the statement after it is genuinely
// unreachable. `control_flow::stmt_diverges` now looks at a `let`'s initializer
// (previously only `exhaustive_return`'s private copy of the rule did).

module Test

func test(foo: !) {
    let x: ! = foo;
    let y: () = (); // WARN: unreachable
}
