// Rosetta Code: Knapsack problem/0-1
// https://rosettacode.org/wiki/Knapsack_problem/0-1
module Main

struct Item {
    let name: String;
    let weight: Int64;
    let value: Int64;
}

@main
func main() {
    let items = [
        Item(name: "map", weight: 9, value: 150), Item(name: "compass", weight: 13, value: 35),
        Item(name: "water", weight: 153, value: 200), Item(name: "sandwich", weight: 50, value: 160),
        Item(name: "glucose", weight: 15, value: 60), Item(name: "tin", weight: 68, value: 45),
        Item(name: "banana", weight: 27, value: 60), Item(name: "apple", weight: 39, value: 40),
        Item(name: "cheese", weight: 23, value: 30), Item(name: "beer", weight: 52, value: 10),
        Item(name: "suntan cream", weight: 11, value: 70), Item(name: "camera", weight: 32, value: 30),
        Item(name: "T-shirt", weight: 24, value: 15), Item(name: "trousers", weight: 48, value: 10),
        Item(name: "umbrella", weight: 73, value: 40), Item(name: "waterproof trousers", weight: 42, value: 70),
        Item(name: "waterproof overclothes", weight: 43, value: 75), Item(name: "note-case", weight: 22, value: 80),
        Item(name: "sunglasses", weight: 7, value: 20), Item(name: "towel", weight: 18, value: 12),
        Item(name: "socks", weight: 4, value: 50), Item(name: "book", weight: 30, value: 10),
    ];
    let capacity = 400;
    let n = items.count;
    // best(i)(w): best value using the first i items within weight w.
    var best = Array(of: n + 1, generatedBy: { (_) in Array[Int64](repeating: 0, count: capacity + 1) });
    for i in 1..=n {
        let item = items(i - 1);
        for w in 0..=capacity {
            best(i)(w) = best(i - 1)(w);
            if item.weight <= w {
                let with = best(i - 1)(w - item.weight) + item.value;
                if with > best(i)(w) { best(i)(w) = with; }
            }
        }
    }
    // Walk back to find which items were taken.
    var w = capacity;
    var taken = Array[Item]();
    var i = n;
    while i > 0 {
        if best(i)(w) != best(i - 1)(w) {
            taken.insert(items(i - 1), at: 0);
            w = w - items(i - 1).weight;
        }
        i = i - 1;
    }
    for item in taken {
        println("\(item.name:<24) \(item.weight:>3) \(item.value:>4)");
    }
    let totalWeight = taken.iter().map { it.weight }.sum();
    println("\("total":<24) \(totalWeight:>3) \(best(n)(capacity):>4)");
}
