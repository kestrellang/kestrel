// Rosetta Code: Day of the week
// https://rosettacode.org/wiki/Day_of_the_week
// In which years between 2008 and 2121 does Christmas fall on a Sunday?
module Main

// Zeller-style congruence: 0 = Sunday.
func dayOfWeek(year: Int64, month: Int64, day: Int64) -> Int64 {
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    (y + y / 4 - y / 100 + y / 400 + t(month - 1) + day) % 7
}

@main
func main() {
    let years = (2008..=2121).iter().filter { dayOfWeek(it, 12, 25) == 0 }.collect();
    println(years.joined(" "));
}
