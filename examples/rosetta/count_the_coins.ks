// Rosetta Code: Count the coins
// https://rosettacode.org/wiki/Count_the_coins
module Main

// ways(a): the number of ways to make amount a from the coins seen so far.
func countWays(amount: Int64, coins: Array[Int64]) -> Int64 {
    var ways = Array[Int64](repeating: 0, count: amount + 1);
    ways(0) = 1;
    for coin in coins {
        for a in coin..=amount {
            ways(a) = ways(a) + ways(a - coin);
        }
    }
    ways(amount)
}

@main
func main() {
    println("100 cents with [1, 5, 10, 25]: \(countWays(100, [1, 5, 10, 25]))");
    println("1000 * 100 cents with [1, 5, 10, 25, 50, 100]: \(countWays(100000, [1, 5, 10, 25, 50, 100]))");
}
