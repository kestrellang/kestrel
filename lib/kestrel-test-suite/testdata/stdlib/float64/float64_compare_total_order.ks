// test: execution
// stdlib: true
// expect-stdout: [-1, 0.5, 2, NaN, NaN]\ntrue\nSome(NaN) Some(-1)\ntrue true true\nfalse false false\n[-1, 0.5, 2, NaN]\n

module Test

// `compare` is a total order: NaN equals NaN and sorts after every number.
// Sorting, min/max and isSorted go through `compare`, so NaN in the data
// no longer leaves `[3, NaN, 1, 2]` "sorted". The `<` family stays IEEE:
// every comparison against NaN is false.
@main
func main() {
    let nan = Float64.nan;
    var a = [2.0, nan, -1.0, nan, 0.5];
    a.sort();
    println(a);
    println(a.isSorted());
    println("\(a.max()) \(a.min())");
    println("\(nan.compare(nan) == .Equal) \(nan.compare(1.0) == .Greater) \((-0.0).compare(0.0) == .Equal)");
    println("\(nan < 1.0) \(nan > 1.0) \(1.0 < nan)");
    var f: Array[Float32] = [2.0, Float32.nan, -1.0, 0.5];
    f.sort();
    println(f);
}
