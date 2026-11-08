// Rosetta Code: Averages/Arithmetic mean, Averages/Median, Averages/Mode,
// Standard deviation
// https://rosettacode.org/wiki/Averages/Arithmetic_mean
module Main

func mean(xs: Array[Float64]) -> Float64 {
    if xs.isEmpty { return 0.0 }
    xs.iter().sum() / Float64(from: xs.count)
}

func median(xs: Array[Float64]) -> Float64 {
    let s = xs.sorted(by: { (a, b) in a < b });
    let n = s.count;
    if n % 2 == 1 { s(n / 2) } else { (s(n / 2 - 1) + s(n / 2)) / 2.0 }
}

func modes(xs: Array[Int64]) -> Array[Int64] {
    var counts = Dictionary[Int64, Int64]();
    for x in xs { counts(x, default: 0) += 1; }
    let top = counts.values.iter().max() ?? 0;
    counts.filter(where: { (_, c) in c == top }).keys.iter().collect().sorted()
}

func standardDeviation(xs: Array[Float64]) -> Float64 {
    let m = mean(xs);
    mean(xs.map { (it - m) * (it - m) }).sqrt()
}

@main
func main() {
    let data = [3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
    println("mean:   \(mean(data))");
    println("median: \(median(data))");
    println("median of [4.1, 5.6, 7.2, 1.7, 9.3, 4.4, 3.2]: \(median([4.1, 5.6, 7.2, 1.7, 9.3, 4.4, 3.2]))");
    println("modes of [1, 3, 6, 6, 6, 6, 7, 7, 12, 12, 17]: \(modes([1, 3, 6, 6, 6, 6, 7, 7, 12, 12, 17]))");
    println("modes of [1, 1, 2, 4, 4]: \(modes([1, 1, 2, 4, 4]))");
    println("std dev of [2, 4, 4, 4, 5, 5, 7, 9]: \(standardDeviation([2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]))");
}
