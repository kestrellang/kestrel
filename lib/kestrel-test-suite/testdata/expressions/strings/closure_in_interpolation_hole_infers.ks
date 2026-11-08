// test: execution
// stdlib: true
// expect-stdout: sum 15\n

// A closure inside an interpolation hole should infer the same way it does in
// a `let`. Today `Int64(from: it.digitValue() ?? 0)` inside the hole fails
// with E109 "expected Int64 got UInt32", while the same expression bound to a
// `let` first compiles and runs.
// Found while writing examples/rosetta (100!).

module Test

@main
func main() {
    let digits = "12345";
    println("sum \(digits.chars.iter().map { Int64(from: it.digitValue() ?? 0) }.sum())");
}
