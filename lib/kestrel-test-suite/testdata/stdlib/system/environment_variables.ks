// test: execution
// stdlib: true

// Environment.setVariable / variable / removeVariable round-trip, and the
// snapshot from variables() sees a variable set in this process.

module Test

@main
func main() -> lang.i64 {
    if Environment.hasVariable("KESTREL_SYSTEM_TEST") { return 1 }
    if Environment.variable("KESTREL_SYSTEM_TEST").isSome() { return 2 }

    match Environment.setVariable("KESTREL_SYSTEM_TEST", to: "a=b c") {
        .Ok(_) => {},
        .Err(_) => { return 3 }
    }
    match Environment.variable("KESTREL_SYSTEM_TEST") {
        .Some(v) => { if not v.isEqual(to: "a=b c") { return 4 } },
        .None => { return 5 }
    }
    match Environment.variableBytes("KESTREL_SYSTEM_TEST") {
        .Some(bytes) => { if bytes.count != 5 { return 6 } },
        .None => { return 7 }
    }

    // The snapshot splits at the first `=`, so the value keeps its own `=`.
    var found = false;
    for pair in Environment.variables() {
        if pair.0.isEqual(to: "KESTREL_SYSTEM_TEST") {
            if not pair.1.isEqual(to: "a=b c") { return 8 }
            found = true
        }
    }
    if not found { return 9 }

    // Setting an empty value keeps the variable set.
    let _ = Environment.setVariable("KESTREL_SYSTEM_TEST", to: "");
    if not Environment.hasVariable("KESTREL_SYSTEM_TEST") { return 10 }

    match Environment.removeVariable("KESTREL_SYSTEM_TEST") {
        .Ok(_) => {},
        .Err(_) => { return 11 }
    }
    if Environment.hasVariable("KESTREL_SYSTEM_TEST") { return 12 }
    // Removing an unset variable succeeds.
    match Environment.removeVariable("KESTREL_SYSTEM_TEST") {
        .Ok(_) => {},
        .Err(_) => { return 13 }
    }
    0
}
