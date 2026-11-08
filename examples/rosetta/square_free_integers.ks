// Rosetta Code: Square-free integers
// https://rosettacode.org/wiki/Square-free_integers
module Main

func isSquareFree(n: Int64) -> Bool {
    var d: Int64 = 2;
    while d * d <= n {
        if n % (d * d) == 0 { return false }
        d = d + 1;
    }
    true
}

@main
func main() {
    let upTo145 = (1..=145).iter().filter { isSquareFree(it) }.collect();
    for row in upTo145.chunks(of: 20).toArray() {
        println(row.map { "\(it:>4)" }.joined(""));
    }
    for limit in [100, 1000, 10000, 100000] {
        println("Square-free numbers up to \(limit): \((1..=limit).iter().filter { isSquareFree(it) }.count())");
    }
}
