// Rosetta Code: Roman numerals/Encode
// https://rosettacode.org/wiki/Roman_numerals/Encode
module Main

func toRoman(n: Int64) -> String {
    let values = [1000, 900, 500, 400, 100, 90, 50, 40, 10, 9, 5, 4, 1];
    let symbols = ["M", "CM", "D", "CD", "C", "XC", "L", "XL", "X", "IX", "V", "IV", "I"];
    var rest = n;
    var result = StringBuilder();
    for (i, value) in values.iter().enumerate() {
        while rest >= value {
            result.append(symbols(i));
            rest = rest - value;
        }
    }
    result.build()
}

@main
func main() {
    for n in [1990, 2008, 1666, 4, 3999, 2024] {
        println("\(n) = \(toRoman(n))");
    }
}
