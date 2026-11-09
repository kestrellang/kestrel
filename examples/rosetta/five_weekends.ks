// Rosetta Code: Five weekends
// https://rosettacode.org/wiki/Five_weekends
// A month has five full weekends when it has 31 days and starts on a Friday.
module Main

func dayOfWeek(year: Int64, month: Int64, day: Int64) -> Int64 {
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    (y + y / 4 - y / 100 + y / 400 + t(month - 1) + day) % 7
}

@main
func main() {
    let names = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let longMonths = [1, 3, 5, 7, 8, 10, 12];
    var found = Array[String]();
    var yearsWithout = Array[Int64]();
    for year in 1900..=2100 {
        var any = false;
        for m in longMonths {
            if dayOfWeek(year, m, 1) == 5 {
                found.append("\(names(m - 1)) \(year)");
                any = true;
            }
        }
        if not any { yearsWithout.append(year); }
    }
    println("\(found.count) months with five weekends");
    println("first five: \(found.prefix(5).toArray().joined(", "))");
    println("last five:  \(found.suffix(5).toArray().joined(", "))");
    println("\(yearsWithout.count) years without one");
}
