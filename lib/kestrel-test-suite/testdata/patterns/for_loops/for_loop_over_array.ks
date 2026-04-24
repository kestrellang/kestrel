// test: diagnostics
// stdlib: true

module Main

func test() {
    var arr = std.collections.Array[std.num.Int64]();
    arr.append(10);
    arr.append(20);
    arr.append(30);

    var sum: std.num.Int64 = 0;
    for item in arr {
        sum = sum + item
    }
}
