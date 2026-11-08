// Rosetta Code: Associative array/Creation
// https://rosettacode.org/wiki/Associative_array/Creation
module Main

@main
func main() {
    var capitals = Dictionary[String, String]();
    capitals.insert("France", "Paris");
    capitals.insert("Japan", "Tokyo");
    capitals.insert("Peru", "Lima");
    println("count: \(capitals.count)");
    println("Japan: \(capitals("Japan") ?? "?")");
    println("Mars: \(capitals("Mars") ?? "unknown")");
    capitals.remove("France");
    println("contains France: \(capitals.contains("France"))");
    // A literal works too.
    let squares: [Int64: Int64] = [1: 1, 2: 4, 3: 9];
    println("3 squared: \(squares(3) ?? 0)");
}
