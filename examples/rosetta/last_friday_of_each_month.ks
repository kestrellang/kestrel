// Rosetta Code: Last Friday of each month
// https://rosettacode.org/wiki/Last_Friday_of_each_month
module Main

func isLeap(y: Int64) -> Bool { (y % 4 == 0 and y % 100 != 0) or y % 400 == 0 }

func daysIn(year: Int64, month: Int64) -> Int64 {
    match month {
        2 => if isLeap(year) { 29 } else { 28 },
        4 or 6 or 9 or 11 => 30,
        _ => 31,
    }
}

// 0 = Sunday ... 6 = Saturday.
func dayOfWeek(year: Int64, month: Int64, day: Int64) -> Int64 {
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    (y + y / 4 - y / 100 + y / 400 + t(month - 1) + day) % 7
}

@main
func main() {
    let year = 2012;
    for month in 1..=12 {
        let last = daysIn(year, month);
        // Step back from the last day to a Friday (5).
        let day = last - (dayOfWeek(year, month, last) - 5 + 7) % 7;
        println("\(year)-\(month:02)-\(day:02)");
    }
}
