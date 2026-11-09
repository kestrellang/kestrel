// Rosetta Code: Number names
// https://rosettacode.org/wiki/Number_names
module Main

let small: Array[String] = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
             "ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen",
             "seventeen", "eighteen", "nineteen"];
let tens: Array[String] = ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];
let scales: Array[String] = ["", " thousand", " million", " billion", " trillion", " quadrillion", " quintillion"];

// Spells 0..<1000.
func belowThousand(n: Int64) -> String {
    if n < 20 { return small(n) }
    if n < 100 {
        let rest = if n % 10 == 0 { "" } else { "-\(small(n % 10))" };
        return "\(tens(n / 10))\(rest)"
    }
    let rest = if n % 100 == 0 { "" } else { " and \(belowThousand(n % 100))" };
    "\(small(n / 100)) hundred\(rest)"
}

func spell(n: Int64) -> String {
    if n == 0 { return "zero" }
    if n < 0 { return "minus \(spell(-n))" }
    var groups = Array[String]();
    var rest = n;
    var scale = 0;
    while rest > 0 {
        let group = rest % 1000;
        if group != 0 { groups.insert("\(belowThousand(group))\(scales(scale))", at: 0); }
        rest = rest / 1000;
        scale = scale + 1;
    }
    groups.joined(by: ", ")
}

@main
func main() {
    for n in [0, 7, 13, 21, 99, 100, 101, 342, 1000, 1001, 12345, 1000000, 123456789, -42,
              9223372036854775807] {
        println("\(n): \(spell(n))");
    }
}
