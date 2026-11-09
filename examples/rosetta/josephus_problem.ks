// Rosetta Code: Josephus problem
// https://rosettacode.org/wiki/Josephus_problem
module Main

// Simulates the circle: returns the kill order; the survivor is last.
func josephus(n: Int64, k: Int64) -> Array[Int64] {
    var circle = Array(of: n, generatedBy: { it });
    var order = Array[Int64]();
    var i = 0;
    while not circle.isEmpty {
        i = (i + k - 1) % circle.count;
        order.append(circle.remove(at: i));
    }
    order
}

@main
func main() {
    let order = josephus(41, 3);
    println("Execution order: \(order.drop(last: 1).toArray().joined(by: " "))");
    println("Survivor: \(order(order.count - 1))");
}
