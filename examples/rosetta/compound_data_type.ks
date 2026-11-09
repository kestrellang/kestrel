// Rosetta Code: Compound data type
// https://rosettacode.org/wiki/Compound_data_type
module Main

struct Point {
    var x: Int64;
    var y: Int64;
}

@main
func main() {
    var p = Point(x: 3, y: 4);
    p.x = p.x + 1;
    println("Point(x: \(p.x), y: \(p.y))");
    // A tuple is the anonymous version.
    let t = (3, 4);
    println("tuple: (\(t.0), \(t.1))");
}
