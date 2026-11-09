// Rosetta Code: Huffman coding
// https://rosettacode.org/wiki/Huffman_coding
module Main

indirect enum Node {
    case Leaf(Char)
    case Branch(Node, Node)
}

// Heap entries are ordered by weight; `order` breaks ties deterministically.
struct Entry {
    let weight: Int64;
    let order: Int64;
    let node: Node;
}

extend Entry: Comparable {
    func isEqual(to o: Entry) -> Bool { self.weight == o.weight and self.order == o.order }
    func compare(o: Entry) -> Ordering {
        if self.weight != o.weight { return self.weight.compare(o.weight) }
        self.order.compare(o.order)
    }
}

func collectCodes(n: Node, prefix: String, mutating out: Array[(Char, String)]) {
    match n {
        .Leaf(c) => out.append((c, prefix)),
        .Branch(l, r) => {
            collectCodes(l, prefix + "0", out);
            collectCodes(r, prefix + "1", out);
        },
    }
}

@main
func main() {
    let text = "this is an example for huffman encoding";
    var counts = Dictionary[Char, Int64]();
    for c in text.chars { counts(c, default: 0) += 1; }
    var heap = Heap[Entry]();
    var order = 0;
    for c in counts.keys.iter().collect().sorted() {
        heap.push(Entry(weight: counts(c) ?? 0, order: order, node: .Leaf(c)));
        order = order + 1;
    }
    while heap.count > 1 {
        let a = heap.pop() ?? Entry(weight: 0, order: 0, node: .Leaf(' '));
        let b = heap.pop() ?? Entry(weight: 0, order: 0, node: .Leaf(' '));
        heap.push(Entry(weight: a.weight + b.weight, order: order, node: .Branch(a.node, b.node)));
        order = order + 1;
    }
    var codes = Array[(Char, String)]();
    if let some root = heap.pop() { collectCodes(root.node, "", codes); }
    codes.sort(byKey: { it.0 });
    var bits = 0;
    for (c, code) in codes {
        let n = counts(c) ?? 0;
        bits = bits + n * code.chars.count;
        println("'\(c)' x\(n:<2) \(code)");
    }
    println("encoded length: \(bits) bits (vs \(text.chars.count * 8) as ASCII)");
}
