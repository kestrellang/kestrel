// Rosetta Code: Loops/Nested
// https://rosettacode.org/wiki/Loops/Nested
module Main

@main
func main() {
    var rng = Lcg64(seed: 99);
    let grid = Array(of: 10, generatedBy: { (_) in
        Array(of: 10, generatedBy: { (_) in Int64.random(in: 1..=20, using: rng) })
    });
    outer: for row in grid {
        for x in row {
            print("\(x:>3)");
            if x == 20 { break outer }
        }
        println();
    }
    println();
}
