// Rosetta Code: FizzBuzz
// https://rosettacode.org/wiki/FizzBuzz
module Main

@main
func main() {
    for i in 1..=100 {
        let line = match (i % 3, i % 5) {
            (0, 0) => "FizzBuzz",
            (0, _) => "Fizz",
            (_, 0) => "Buzz",
            _ => "\(i)",
        };
        println(line);
    }
}
