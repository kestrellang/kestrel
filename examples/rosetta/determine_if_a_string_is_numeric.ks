// Rosetta Code: Determine if a string is numeric
// https://rosettacode.org/wiki/Determine_if_a_string_is_numeric
module Main

func isNumeric(s: String) -> Bool {
    if let .Some(_) = Int64(parsing: s) { return true }
    if let .Some(_) = Float64(parsing: s) { return true }
    false
}

@main
func main() {
    for s in ["123", "-45", "3.14", "1e10", "abc", "12a", "", " 7", "0x1F"] {
        println("\"\(s)\" numeric: \(isNumeric(s))");
    }
}
