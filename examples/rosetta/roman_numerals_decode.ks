// Rosetta Code: Roman numerals/Decode
// https://rosettacode.org/wiki/Roman_numerals/Decode
module Main

func value(of c: Char) -> Int64 {
    match c {
        'I' => 1, 'V' => 5, 'X' => 10, 'L' => 50,
        'C' => 100, 'D' => 500, 'M' => 1000,
        _ => 0,
    }
}

// A symbol smaller than the one after it is subtracted (the I in IV).
func fromRoman(s: String) -> Int64 {
    let digits = s.chars.iter().map { value(of: it) }.collect();
    var total = 0;
    for (i, d) in digits.iter().enumerate() {
        if i + 1 < digits.count and d < digits(i + 1) {
            total = total - d;
        } else {
            total = total + d;
        }
    }
    total
}

@main
func main() {
    for s in ["MCMXC", "MMVIII", "MDCLXVI", "IV", "MMMCMXCIX"] {
        println("\(s) = \(fromRoman(s))");
    }
}
