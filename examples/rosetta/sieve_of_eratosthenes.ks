// Rosetta Code: Sieve of Eratosthenes
// https://rosettacode.org/wiki/Sieve_of_Eratosthenes
module Main

func primes(upTo limit: Int64) -> Array[Int64] {
    var isPrime = Array[Bool](repeating: true, count: limit + 1);
    isPrime(0) = false;
    isPrime(1) = false;
    var i = 2;
    while i * i <= limit {
        if isPrime(i) {
            var j = i * i;
            while j <= limit {
                isPrime(j) = false;
                j = j + i;
            }
        }
        i = i + 1;
    }
    (0..=limit).iter().filter { isPrime(it) }.collect()
}

@main
func main() {
    println(primes(upTo: 100));
    println("There are \(primes(upTo: 1000000).count) primes below one million");
}
