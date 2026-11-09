// Rosetta Code: Increment a numerical string
// https://rosettacode.org/wiki/Increment_a_numerical_string
module Main

@main
func main() {
    for s in ["12345", "-1", "9223372036854775806"] {
        match Int64(parsing: s) {
            some n => println("\(s) + 1 = \(n + 1)"),
            null => println("\(s) is not a number"),
        }
    }
}
