// Rosetta Code: N'th
// https://rosettacode.org/wiki/N%27th
module Main

func ordinal(n: Int64) -> String {
    let suffix = if n % 100 >= 11 and n % 100 <= 13 {
        "th"
    } else {
        match n % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    };
    "\(n)'\(suffix)"
}

@main
func main() {
    for range in [0..<26, 250..<266, 1000..<1026] {
        println(range.iter().map { ordinal(it) }.collect().joined(" "));
    }
}
