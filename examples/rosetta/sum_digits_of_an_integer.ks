// Rosetta Code: Sum digits of an integer
// https://rosettacode.org/wiki/Sum_digits_of_an_integer
module Main

func digitSum(n: Int64, base: Int64) -> Int64 {
    var rest = n;
    var sum: Int64 = 0;
    while rest > 0 {
        sum = sum + rest % base;
        rest = rest / base;
    }
    sum
}

@main
func main() {
    println("1 (base 10) -> \(digitSum(1, 10))");
    println("1234 (base 10) -> \(digitSum(1234, 10))");
    println("0xfe (base 16) -> \(digitSum(0xfe, 16))");
    println("0xf0e (base 16) -> \(digitSum(0xf0e, 16))");
}
