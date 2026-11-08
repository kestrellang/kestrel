// Rosetta Code: Equilibrium index
// https://rosettacode.org/wiki/Equilibrium_index
module Main

func equilibria(a: Array[Int64]) -> Array[Int64] {
    let total = a.iter().sum();
    var left: Int64 = 0;
    var result = Array[Int64]();
    for (i, x) in a.iter().enumerate() {
        if left == total - left - x { result.append(i); }
        left = left + x;
    }
    result
}

@main
func main() {
    println(equilibria([-7, 1, 5, 2, -4, 3, 0]));
    println(equilibria([2, 4, 6]));
    println(equilibria([2, 9, 2]));
    println(equilibria([1, -1, 1, -1, 1, -1, 1]));
}
