// Rosetta Code: ISBN13 check digit
// https://rosettacode.org/wiki/ISBN13_check_digit
module Main

func isValidIsbn13(isbn: String) -> Bool {
    let digits = isbn.chars.iter().filterMap(as: { it.digitValue() }).collect();
    if digits.count != 13 { return false }
    var sum: Int64 = 0;
    for (i, d) in digits.iter().enumerate() {
        sum = sum + Int64(from: d) * (if i % 2 == 0 { 1 } else { 3 });
    }
    sum % 10 == 0
}

@main
func main() {
    for isbn in ["978-0596528126", "978-0596528120", "978-1788399081", "978-1788399083"] {
        println("\(isbn): \(if isValidIsbn13(isbn) { "good" } else { "bad" })");
    }
}
