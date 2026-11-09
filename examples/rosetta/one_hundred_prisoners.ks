// Rosetta Code: 100 prisoners
// https://rosettacode.org/wiki/100_prisoners
module Main

// Random search: each prisoner opens 50 random drawers.
func randomStrategy(drawers: Array[Int64], mutating rng: Lcg64) -> Bool {
    for prisoner in 0..<100 {
        var order = Array(from: 0..<100);
        order.shuffle(using: rng);
        if not order.prefix(50).contains(prisoner) { return false }
    }
    true
}

// Follow the cycle: open your own number, then the number found inside.
func cycleStrategy(drawers: Array[Int64]) -> Bool {
    for prisoner in 0..<100 {
        var drawer = prisoner;
        var found = false;
        for _ in 0..<50 {
            if drawers(drawer) == prisoner {
                found = true;
                break
            }
            drawer = drawers(drawer);
        }
        if not found { return false }
    }
    true
}

@main
func main() {
    var rng = Lcg64(seed: 100);
    let trials = 2000;
    var randomWins = 0;
    var cycleWins = 0;
    for _ in 0..<trials {
        var drawers = Array(from: 0..<100);
        drawers.shuffle(using: rng);
        if randomStrategy(drawers, rng) { randomWins = randomWins + 1; }
        if cycleStrategy(drawers) { cycleWins = cycleWins + 1; }
    }
    println("Random strategy: \(randomWins) / \(trials) wins");
    println("Cycle strategy:  \(cycleWins) / \(trials) wins (\(Float64(from: cycleWins) / Float64(from: trials):%))");
}
