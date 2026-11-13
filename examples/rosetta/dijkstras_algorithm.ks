// Rosetta Code: Dijkstra's algorithm
// https://rosettacode.org/wiki/Dijkstra%27s_algorithm
module Main

struct Edge {
    let from: String;
    let to: String;
    let cost: Int64;
}

// A frontier entry, ordered by distance so the Heap pops the nearest first.
struct Visit {
    let dist: Int64;
    let node: String;
}

extend Visit: Comparable {
    func isEqual(to other: Visit) -> Bool { self.dist == other.dist }
    func compare(other: Visit) -> Ordering { self.dist.compare(other.dist) }
}

func shortestPath(edges: Array[Edge], source: String, target: String) -> (Int64, Array[String]) {
    var dist = Dictionary[String, Int64]();
    var prev = Dictionary[String, String]();
    var frontier = Heap[Visit]();
    dist.insert(source, 0);
    frontier.push(Visit(dist: 0, node: source));
    while let some visit = frontier.pop() {
        if visit.dist > (dist(visit.node) ?? visit.dist) { continue }
        for edge in edges {
            if edge.from != visit.node { continue }
            let candidate = visit.dist + edge.cost;
            if candidate < (dist(edge.to) ?? (candidate + 1)) {
                dist.insert(edge.to, candidate);
                prev.insert(edge.to, visit.node);
                frontier.push(Visit(dist: candidate, node: edge.to));
            }
        }
    }
    var path = [target];
    var node = target;
    while let some p = prev(node) {
        path.insert(p, at: 0);
        node = p;
    }
    (dist(target) ?? -1, path)
}

@main
func main() {
    let edges = [
        Edge(from: "a", to: "b", cost: 7), Edge(from: "a", to: "c", cost: 9),
        Edge(from: "a", to: "f", cost: 14), Edge(from: "b", to: "c", cost: 10),
        Edge(from: "b", to: "d", cost: 15), Edge(from: "c", to: "d", cost: 11),
        Edge(from: "c", to: "f", cost: 2), Edge(from: "d", to: "e", cost: 6),
        Edge(from: "e", to: "f", cost: 9),
    ];
    for target in ["e", "f"] {
        let (cost, path) = shortestPath(edges, "a", target);
        println("a -> \(target): \(path.joined(by: " -> ")) (cost \(cost))");
    }
}
