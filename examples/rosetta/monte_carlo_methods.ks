// Rosetta Code: Monte Carlo methods
// https://rosettacode.org/wiki/Monte_Carlo_methods
module Main

func estimatePi(samples: Int64, mutating rng: Lcg64) -> Float64 {
    var inside: Int64 = 0;
    for _ in 0..<samples {
        let x = Float64.random(using: rng);
        let y = Float64.random(using: rng);
        if x * x + y * y <= 1.0 { inside = inside + 1; }
    }
    4.0 * Float64(from: inside) / Float64(from: samples)
}

@main
func main() {
    // A seeded generator keeps the run reproducible.
    var rng = Lcg64(seed: 12345);
    for samples in [1000, 10000, 100000, 1000000] {
        let estimate = estimatePi(samples, rng);
        println("\(samples:>8) samples: pi ~ \(estimate:.4), error \((estimate - Float64.pi).abs():.4)");
    }
}
