// Rosetta Code: Topological sort
// https://rosettacode.org/wiki/Topological_sort
module Main

// Kahn's algorithm. Ties are broken alphabetically so the order is stable.
func topoSort(deps: Array[(String, Array[String])]) -> Result[Array[String], String] {
    var indegree = Dictionary[String, Int64]();
    var users = Dictionary[String, Array[String]]();
    for (item, needs) in deps {
        indegree(item, default: 0) += 0;
        for need in needs {
            if need == item { continue }   // self-dependencies are ignored
            indegree(need, default: 0) += 0;
            indegree(item, default: 0) += 1;
            users(need, default: []).append(item);
        }
    }
    var ready = indegree.filter { (_, n) in n == 0 }.keys.iter().collect().sorted();
    var order = Array[String]();
    while not ready.isEmpty {
        let next = ready.remove(at: 0);
        order.append(next);
        for user in users(next) ?? [] {
            indegree(user, default: 0) -= 1;
            if indegree(user) == .Some(0) { ready.append(user); }
        }
        ready.sort();
    }
    if order.count != indegree.count { return .Err("dependency cycle detected") }
    .Ok(order)
}

@main
func main() {
    let deps: Array[(String, Array[String])] = [
        ("des_system_lib", ["std", "synopsys", "std_cell_lib", "des_system_lib", "dw02", "dw01", "ramlib", "ieee"]),
        ("dw01", ["ieee", "dw01", "dware", "gtech"]),
        ("dw02", ["ieee", "dw02", "dware"]),
        ("dw03", ["std", "synopsys", "dware", "dw03", "dw02", "dw01", "ieee", "gtech"]),
        ("dw04", ["dw04", "ieee", "dw01", "dware", "gtech"]),
        ("dw05", ["dw05", "ieee", "dware"]),
        ("dw06", ["dw06", "ieee", "dware"]),
        ("dw07", ["ieee", "dware"]),
        ("dware", ["ieee", "dware"]),
        ("gtech", ["ieee", "gtech"]),
        ("ramlib", ["std", "ieee"]),
        ("std_cell_lib", ["ieee", "std_cell_lib"]),
        ("synopsys", []),
    ];
    match topoSort(deps) {
        .Ok(order) => println(order.joined(" ")),
        .Err(e) => println(e),
    }
    // Adding dw04 as a dependency of dw01 creates a cycle.
    var cyclic = deps;
    cyclic(1) = ("dw01", ["ieee", "dw01", "dware", "gtech", "dw04"]);
    match topoSort(cyclic) {
        .Ok(order) => println(order.joined(" ")),
        .Err(e) => println(e),
    }
}
