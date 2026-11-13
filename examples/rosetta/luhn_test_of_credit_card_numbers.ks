// Rosetta Code: Luhn test of credit card numbers
// https://rosettacode.org/wiki/Luhn_test_of_credit_card_numbers
module Main

func luhn(number: String) -> Bool {
    var sum = 0;
    for (i, c) in number.chars.reversed.iter().enumerate() {
        let d = Int64(from: c.digitValue() ?? 0);
        if i % 2 == 0 {
            sum = sum + d;
        } else {
            let doubled = d * 2;
            sum = sum + doubled / 10 + doubled % 10;
        }
    }
    sum % 10 == 0
}

@main
func main() {
    for n in ["49927398716", "49927398717", "1234567812345678", "1234567812345670"] {
        println("\(n): \(if luhn(n) { "valid" } else { "invalid" })");
    }
}
