// Rosetta Code: Monty Hall problem
// https://rosettacode.org/wiki/Monty_Hall_problem
module Main

@main
func main() {
    var rng = Lcg64(seed: 3);
    let games = 100000;
    var stayWins = 0;
    var switchWins = 0;
    for _ in 0..<games {
        let car = Int64.random(in: 0..<3, using: rng);
        let pick = Int64.random(in: 0..<3, using: rng);
        // The host opens a goat door; switching wins exactly when the first pick was wrong.
        if pick == car { stayWins = stayWins + 1; } else { switchWins = switchWins + 1; }
    }
    println("Staying wins   \(Float64(from: stayWins) / Float64(from: games):.1%) of the time");
    println("Switching wins \(Float64(from: switchWins) / Float64(from: games):.1%) of the time");
}
