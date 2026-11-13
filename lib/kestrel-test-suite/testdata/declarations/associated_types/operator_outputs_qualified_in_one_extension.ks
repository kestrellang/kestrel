// test: execution
// stdlib: true
// expect-stdout: 15\n12\n
//
// `Addable` and `Subtractable` both declare `Output`. Qualified bindings let
// one extension conform to both with different outputs.

module Test

struct Stamp { let v: Int64 }
struct Span { let v: Int64 }

extend Stamp: Addable[Span], Subtractable[Stamp] {
    type Addable.Output = Stamp
    type Subtractable.Output = Span
    consuming func add(consuming other: Span) -> Stamp { Stamp(v: self.v + other.v) }
    consuming func subtract(consuming other: Stamp) -> Span { Span(v: self.v - other.v) }
}

@main
func main() {
    let later: Stamp = Stamp(v: 10) + Span(v: 5);
    let gap: Span = later - Stamp(v: 3);
    println(later.v);
    println(gap.v);
}
