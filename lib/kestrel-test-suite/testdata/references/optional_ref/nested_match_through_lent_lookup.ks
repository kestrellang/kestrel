// test: execution
// stdlib: true
// expect-stdout: 2 3\n

// Matching a nested pattern straight through a lent `Optional[&V]`
// (`d(key)` on a dictionary of enums) copies the inner value out, like the
// flat `.Some(v) => v` case: the decision tree ends the lent `&V` after
// reading its tag instead of holding it across the arms (was E497).
module Test

import std.collections.(Array, Dictionary)

enum V {
    case Obj(Array[Int64])
    case Num(Int64)
}

@main
func main() -> lang.i64 {
    var root: Dictionary[String, V] = [:];
    root.insert("t", .Obj([1, 2]));
    var obj: Array[Int64] = match root("t") {
        some .Obj(existing) => existing,
        _ => []
    };
    let n = obj.count;
    obj.append(3);
    print("\(n) \(obj.count)\n");
    0
}
