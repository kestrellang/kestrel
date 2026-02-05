import std.collections.Array
import std.result.Optional
import std.num.Int64

func main() -> Int64 {
    var arrOpt = Array[Optional[Int64]]();
    arrOpt.append(.Some(1));
    var compactIter = arrOpt.iter().compactMap();
    let first = compactIter.next();
    if let .Some(val) = first {
        if val != 1 { return 1 }
    }
    return 0
}
