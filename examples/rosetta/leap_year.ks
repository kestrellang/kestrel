// Rosetta Code: Leap year
// https://rosettacode.org/wiki/Leap_year
module Main

func isLeapYear(year: Int64) -> Bool {
    (year % 4 == 0 and year % 100 != 0) or year % 400 == 0
}

@main
func main() {
    for year in [1900, 1994, 1996, 1997, 2000, 2024, 2100] {
        let kind = if isLeapYear(year) { "is" } else { "is not" };
        println("\(year) \(kind) a leap year");
    }
}
