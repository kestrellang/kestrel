// Rosetta Code: Floyd-Warshall algorithm
// https://rosettacode.org/wiki/Floyd-Warshall_algorithm
module Main

@main
func main() {
    let n = 4;
    let inf: Int64 = 1 << 40;
    var dist = Array(of: n, generatedBy: { (i) in
        Array(of: n, generatedBy: { (j) in if i == j { 0 } else { inf } })
    });
    var next = Array(of: n, generatedBy: { (_) in Array[Int64](repeating: -1, count: n) });
    // Edges (from, to, weight), 1-based as in the task.
    for (u, v, w) in [(1, 3, -2), (3, 4, 2), (4, 2, -1), (2, 1, 4), (2, 3, 3)] {
        dist(u - 1)(v - 1) = w;
        next(u - 1)(v - 1) = v - 1;
    }
    for k in 0..<n {
        for i in 0..<n {
            for j in 0..<n {
                if dist(i)(k) + dist(k)(j) < dist(i)(j) {
                    dist(i)(j) = dist(i)(k) + dist(k)(j);
                    next(i)(j) = next(i)(k);
                }
            }
        }
    }
    println("pair     dist    path");
    for i in 0..<n {
        for j in 0..<n {
            if i == j { continue }
            var path = [i + 1];
            var u = i;
            while u != j {
                u = next(u)(j);
                path.append(u + 1);
            }
            println("\(i + 1) -> \(j + 1)  \(dist(i)(j):>4)     \(path.joined(by: " -> "))");
        }
    }
}
