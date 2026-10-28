// test: diagnostics
// stdlib: false

// The `inner: loop` below the `break` genuinely never runs: `break` is
// `!`-typed regardless of whether its label resolves, so dead-code analysis
// warns on the statement after it (G12 dropped the `in_loop`/labeled-break
// carve-out). The undeclared-label error is a separate, earlier diagnostic.

module Main

func test() {
    while true {
        break inner; // ERROR: undeclared label
        inner: loop { // WARN: unreachable
            break;
        }
    }
}
