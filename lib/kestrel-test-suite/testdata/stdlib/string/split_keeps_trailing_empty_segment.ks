// test: execution
// stdlib: true
// expect-exit: 0

// A separator at either end of the string produces an empty segment, for
// both `split(_:)` and `split(where:)`. The trailing one used to be dropped
// while the leading one was kept (`",a"` -> 2 segments, `"a,"` -> 1).
// Array `split` already kept it.
module Test

func segments(s: String) -> String {
    var out = "";
    for part in s.split(",") { out.append("<"); out.append(part.toOwned()); out.append(">"); }
    out
}

func segmentsWhere(s: String) -> String {
    var out = "";
    for part in s.split(where: { (c) in c == ',' }) { out.append("<"); out.append(part.toOwned()); out.append(">"); }
    out
}

@main
func main() -> Int64 {
    if segments("a,") != "<a><>" { return 1 }
    if segments(",a") != "<><a>" { return 2 }
    if segments("a,,b,") != "<a><><b><>" { return 3 }
    if segments(",") != "<><>" { return 4 }
    if segments("") != "" { return 5 }
    if segments("a") != "<a>" { return 6 }
    if segmentsWhere("a,") != "<a><>" { return 7 }
    if segmentsWhere(",") != "<><>" { return 8 }
    if "a,b,".split(",").count != 3 { return 9 }
    0
}
