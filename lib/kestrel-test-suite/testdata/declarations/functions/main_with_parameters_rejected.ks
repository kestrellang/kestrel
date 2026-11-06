// test: diagnostics
// executable: true

// The entry point is called with no arguments.

module Test

@main
func main(n: Int64) -> Int64 { // ERROR: cannot take parameters
    n
}
