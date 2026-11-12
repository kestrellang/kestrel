// test: execution
// stdlib: true
// expect-stdout: hello\nhello world\nhello world\nhello world\nhello world\n2 he\n

// A borrowed `&StringSlice` converts to an owned `String` wherever the
// target names `String` (`DecaysFrom[StringSlice]`): an annotation, a
// consuming parameter, a field, a return type, an assignment, an annotated
// collection element.

module Test

import std.numeric.(Int64)
import std.collections.(Array)

struct Named {
    var name: String
}

func keep(consuming x: String) -> String { x }

func trimmedCopy(s: String) -> String { s.trimmed() }

@main
func main() {
    let s = "  hello world  ";
    let a: String = s.subslice(from: 2, to: 7);
    println(a);
    println(keep(s.trimmed()));
    let n = Named(name: s.trimmed());
    println(n.name);
    println(trimmedCopy(s));
    var t: String = "";
    t = s.trimmed();
    println(t);
    let xs: Array[String] = [s.trimmed(), s.subslice(from: 2, to: 4)];
    println("\(xs.count) \(xs(1))");
}
