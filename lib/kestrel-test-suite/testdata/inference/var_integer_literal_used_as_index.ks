// test: execution
// stdlib: true
// expect-stdout: 25\n

// An unannotated `var i = 2` used as an array index (and in arithmetic with
// an Int64) should default to Int64. Today the subscript's generic index
// parameter stays open: E105 "cannot infer type parameter 'I'" and
// "integer literal !: Multipliable" on `i * i`.
// Found while writing examples/rosetta (Sieve of Eratosthenes).

module Test

func countPrimes(limit: Int64) -> Int64 {
    var composite = Array[Bool](repeating: false, count: limit + 1);
    var i = 2;
    while i * i <= limit {
        var j = i * i;
        while j <= limit {
            composite(j) = true;
            j = j + i;
        }
        i = i + 1;
    }
    (2..=limit).iter().filter { composite(it) == false }.count()
}

@main
func main() {
    println(countPrimes(100));
}
