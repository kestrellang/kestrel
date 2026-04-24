// test: diagnostics
// stdlib: true

module Main

struct Pair {
    let first: std.num.Int
    let second: std.num.Int
}

func test() {
    var arr = std.collections.Array[Pair]();
    arr.append(Pair(first: 1, second: 2));
    arr.append(Pair(first: 3, second: 4));

    var sum: std.num.Int = 0;
    for pair in arr {
        sum = sum + pair.first + pair.second
    }
}
