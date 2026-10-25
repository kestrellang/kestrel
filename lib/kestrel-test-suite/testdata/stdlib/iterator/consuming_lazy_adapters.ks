// test: execution
// stdlib: true
// backends: cranelift,llvm

module Test

struct Counter: Iterator, not Copyable {
    type Item = Int64
    var current: Int64
    var end: Int64

    mutating func next() -> Int64? {
        if self.current >= self.end { return .None };
        let value = self.current;
        self.current = self.current + 1;
        .Some(value)
    }
}

struct Resource: not Copyable {
    var value: Int64
}

@main
func main() -> lang.i64 {
    let values: [Int64] = Counter(current: 1, end: 4)
        .filter(where: { it % 2 == 1 })
        .map(as: { it * 10 })
        .collect();
    if values.count != 2 { return 1 }
    if values(unchecked: 0) != 10 { return 2 }
    if values(unchecked: 1) != 30 { return 3 }

    var single = std.iter.once(Resource(value: 99));
    match single.next() {
        .Some(value) => if value.value != 99 { return 4 },
        .None => return 5
    };
    if single.next().isSome() { return 6 }

    0
}
