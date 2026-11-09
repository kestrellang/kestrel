// Rosetta Code: Twin primes
// https://rosettacode.org/wiki/Twin_primes
module Main

@main
func main() {
    let limit: Int64 = 10000000;
    var composite = Array[Bool](repeating: false, count: limit + 3);
    var i: Int64 = 2;
    while i * i <= limit + 2 {
        if composite(i) == false {
            var j = i * i;
            while j <= limit + 2 {
                composite(j) = true;
                j = j + i;
            }
        }
        i = i + 1;
    }
    var power: Int64 = 10;
    var count: Int64 = 0;
    for p in 3..<limit {
        if composite(p) == false and composite(p + 2) == false and p + 2 < power {
            count = count + 1;
        }
        if p + 1 == power {
            println("Twin prime pairs below \(power): \(count)");
            power = power * 10;
        }
    }
}
