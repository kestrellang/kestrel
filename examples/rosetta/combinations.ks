// Rosetta Code: Combinations
// https://rosettacode.org/wiki/Combinations
module Main

func combinations(n: Int64, k: Int64, start: Int64, mutating current: Array[Int64]) {
    if current.count == k {
        println(current.joined(" "));
        return
    }
    for i in start..<n {
        current.append(i);
        combinations(n, k, i + 1, current);
        current.pop();
    }
}

@main
func main() {
    var current = Array[Int64]();
    combinations(5, 3, 0, current);
}
