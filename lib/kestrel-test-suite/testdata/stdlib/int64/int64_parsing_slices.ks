// test: execution
// stdlib: true
// expect-stdout: 78 99 7.5 255\n

module Test

// Int64(parsing:) and Float64(parsing:) take any string view, so split
// pieces parse without being copied into a String first.
@main
func main() {
    let s = "123 -45 7.5 ff";
    var total: Int64 = 0;
    for w in s.split(" ") {
        if let .Some(n) = Int64(parsing: w) { total = total + n }
    }
    let a = Int64(parsing: "99") ?? 0;
    let pieces = s.split(" ").collect();
    let b = Float64(parsing: pieces(2)) ?? 0.0;
    let c = Int64(parsing: "ff", radix: 16) ?? 0;
    println("\(total) \(a) \(b) \(c)");
}
