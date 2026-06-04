// test: execution
// stdlib: true
// expect-exit: 0

module Test

@main
func main() -> lang.i64 {
    let name = "World";

    // No annotation — should default to String via literal defaults
    let greeting = "Hello, \(name)!";
    if greeting != "Hello, World!" { return 1 }

    // Explicit String annotation — same behavior
    let explicit: String = "value is \(42)";
    if explicit != "value is 42" { return 2 }

    // Nested interpolation
    let a = 1;
    let b = 2;
    let expr = "\(a) + \(b) = \(a + b)";
    if expr != "1 + 2 = 3" { return 3 }

    0
}
