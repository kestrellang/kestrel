// Rosetta Code: Middle three digits
// https://rosettacode.org/wiki/Middle_three_digits
module Main

func middleThree(n: Int64) -> Result[String, String] {
    let digits = "\(n.abs())";
    let len = digits.chars.count;
    if len < 3 { return .Err("too short") }
    if len % 2 == 0 { return .Err("even number of digits") }
    let start = (len - 3) / 2;
    .Ok(digits.chars.iter().skip(start).take(3).collect().joined(""))
}

@main
func main() {
    let inputs = [123, 12345, 1234567, 987654321, 10001, -10001, -123, -100, 100, -12345,
                  1, 2, -1, -10, 2002, -2002, 0];
    for n in inputs {
        match middleThree(n) {
            .Ok(m) => println("\(n:>10): \(m)"),
            .Err(e) => println("\(n:>10): error, \(e)"),
        }
    }
}
