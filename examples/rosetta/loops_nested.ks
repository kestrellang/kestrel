// Rosetta Code: Loops/Nested
// https://rosettacode.org/wiki/Loops/Nested
module Main

@main
func main() {
    var rng = Lcg64(seed: 99);
    // `generatedBy` takes a read-only callback, so the generator (which
    // `random` mutates) is driven from a plain loop.
    var grid = Array[Array[Int64]]();
    for _ in 0..<10 {
        var row = Array[Int64]();
        for _ in 0..<10 {
            row.append(Int64.random(in: 1..=20, using: rng));
        }
        grid.append(row);
    }
    outer: for row in grid {
        for x in row {
            print("\(x:>3)");
            if x == 20 { break outer }
        }
        println();
    }
    println();
}
