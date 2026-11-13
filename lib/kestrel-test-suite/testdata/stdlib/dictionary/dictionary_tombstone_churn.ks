// test: execution
// stdlib: true
// expect-stdout: 1000 true 1000 0\n

module Test

// Removed entries leave tombstones. They count toward the load factor, so
// under insert/remove churn the table rehashes in place instead of filling
// with tombstones until every lookup probes the whole table (100k churning
// ops once took minutes). The capacity stays near what the live entries need.
@main
func main() {
    var d = Dictionary[Int64, Int64]();
    for i in 0..<100000 {
        d.insert(i, i);
        if i >= 1000 { d.remove(i - 1000); }
    }
    var hits = 0;
    var misses = 0;
    for i in 0..<100000 {
        if d.contains(i) { hits = hits + 1 } else if i >= 99000 { misses = misses + 1 }
    }
    println("\(d.count) \(d.capacity <= 4096) \(hits) \(misses)");
}
