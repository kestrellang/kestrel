// test: execution
// stdlib: true
// expect-exit: 0
//
// Reading a property requirement directly on `any P` calls the payload's
// getter through the function table. It used to lower as a field read on
// the raw existential, which returned a pointer for `Int64` and crashed
// for `String`. Stored and computed conformances both dispatch.

module Test

import std.numeric.(Int64)
import std.text.(String)
import std.collections.(Array)

protocol Named {
    var name: String { get }
    var size: Int64 { get }
    var tags: Array[String] { get }
}

struct Stored: Named {
    let name: String;
    let size: Int64;
    let tags: Array[String];
}

struct Computed: Named {
    let base: Int64;
    var name: String { "computed" }
    var size: Int64 { self.base * 2 }
    var tags: Array[String] { ["a", "b"] }
}

@main
func main() -> Int64 {
    let s: any Named = Stored(name: "stored", size: 7, tags: ["x"]);
    if s.name != "stored" { return 1 }
    if s.size != 7 { return 2 }
    if s.tags.count != 1 { return 3 }
    let items: Array[any Named] = [s, Computed(base: 21)];
    var total: Int64 = 0;
    for item in items { total = total + item.size + item.tags.count; }
    if total != 7 + 1 + 42 + 2 { return 4 }
    if items(1).name != "computed" { return 5 }
    0
}
