// Rosetta Code: Harshad or Niven series
// https://rosettacode.org/wiki/Harshad_or_Niven_series
module Main

func digitSum(n: Int64) -> Int64 {
    var rest = n;
    var sum = 0;
    while rest > 0 {
        sum = sum + rest % 10;
        rest = rest / 10;
    }
    sum
}

func isHarshad(n: Int64) -> Bool { n % digitSum(n) == 0 }

@main
func main() {
    let first20 = (1..).iter().filter { isHarshad(it) }.take(20).collect();
    println("First 20: \(first20)");
    let after1000 = (1001..).iter().find { isHarshad(it) };
    println("First above 1000: \(after1000 ?? 0)");
}
