// test: diagnostics
// stdlib: true

module Main

struct Pair {
    let first: std.numeric.Int
    let second: std.numeric.Int
}

func test() {
    var arr = std.collections.Array[Pair]();
    arr.append(Pair(first: 1, second: 2));
    arr.append(Pair(first: 3, second: 4));

    var sum: std.numeric.Int = 0;
    for pair in arr {
        sum = sum + pair.first + pair.second
    }
}
