// Rosetta Code: Twin primes
// https://rosettacode.org/wiki/Twin_primes
module Main

@main
func main() {
    let limit: Int64 = 10000000;
    var composite = Array[Bool](repeating: false, count: limit + 3);
    var i = 2;
    while i * i <= limit + 2 {
        if not composite(i) {
            var j = i * i;
            while j <= limit + 2 {
                composite(j) = true;
                j = j + i;
            }
        }
        i = i + 1;
    }
    var power = 10;
    var count = 0;
    for p in 3..<limit {
        if not composite(p) and not composite(p + 2) and p + 2 < power {
            count = count + 1;
        }
        if p + 1 == power {
            println("Twin prime pairs below \(power): \(count)");
            power = power * 10;
        }
    }
}
