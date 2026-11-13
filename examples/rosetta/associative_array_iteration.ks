// Rosetta Code: Associative array/Iteration
// https://rosettacode.org/wiki/Associative_array/Iteration
module Main

@main
func main() {
    let ages: [String: Int64] = ["alice": 31, "bob": 27, "carol": 45];
    // Dictionary order is unspecified, so sort the keys for stable output.
    let names = ages.keys.iter().collect().sorted();
    for name in names {
        println("\(name) -> \(ages(name) ?? 0)");
    }
    println("keys:   \(names)");
    println("values: \(ages.values.iter().collect().sorted())");
}
