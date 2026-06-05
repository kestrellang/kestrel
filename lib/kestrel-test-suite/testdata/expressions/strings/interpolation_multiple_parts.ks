// test: execution
// stdlib: true
// expect-exit: 0

module Test

@main
func main() -> lang.i64 {
    let a = "one";
    let b = "two";
    let c = "three";

    if "\(a), \(b), \(c)" != "one, two, three" { return 1 }

    if "start \(a) middle \(b) end" != "start one middle two end" { return 2 }

    // Adjacent interpolations with no literal between them
    if "\(a)\(b)\(c)" != "onetwothree" { return 3 }

    // Interpolation-only string (no literal parts at all)
    if "\(a)" != "one" { return 4 }

    0
}
